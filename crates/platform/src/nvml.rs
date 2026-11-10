//! NVIDIA VRAM through NVML (`libnvidia-ml.so.1`, part of NVIDIA's driver), for the resource
//! manager (ACS-3, `cfa4-tasks`).
//!
//! NVIDIA's proprietary driver publishes no VRAM counters in sysfs, so `cfa4-accel` leaves the
//! card's total and used VRAM unknown. This module reads them from NVML, keyed by PCI address,
//! and [`reports`] ties them to render nodes as [`GpuMemoryReport`]s; the caller applies them to
//! its snapshot (`Snapshot::apply_gpu_report`). NVML is loaded at run time: without the driver
//! there is simply no report, never a failure to start. Only what the manager needs is read
//! (memory and PCI address); NVML is initialised and shut down around each reading.

use cfa4_accel::memory::{PciAddress, PciMemoryReading};
use cfa4_accel::{GpuMemoryReport, Inventory};

/// At most this many NVML devices are read (a hostile count is not trusted).
const MAX_DEVICES: u32 = 64;

type Return = i32;
type Device = *mut std::ffi::c_void;
const SUCCESS: Return = 0;

/// `nvmlMemory_t` (NVML 8 and later).
#[repr(C)]
#[derive(Clone, Copy, Default)]
#[allow(dead_code)] // the C layout: NVML writes every field, not all are read
struct Memory {
    total: u64,
    free: u64,
    used: u64,
}

/// `nvmlPciInfo_t` as `nvmlDeviceGetPciInfo_v3` fills it.
#[repr(C)]
#[derive(Clone, Copy)]
#[allow(dead_code)] // the C layout: NVML writes every field, not all are read
struct PciInfo {
    bus_id_legacy: [u8; 16],
    domain: u32,
    bus: u32,
    device: u32,
    pci_device_id: u32,
    pci_sub_system_id: u32,
    bus_id: [u8; 32],
}

type Init = unsafe extern "C" fn() -> Return;
type Shutdown = unsafe extern "C" fn() -> Return;
type DeviceGetCount = unsafe extern "C" fn(*mut u32) -> Return;
type DeviceGetHandleByIndex = unsafe extern "C" fn(u32, *mut Device) -> Return;
type DeviceGetMemoryInfo = unsafe extern "C" fn(Device, *mut Memory) -> Return;
type DeviceGetPciInfo = unsafe extern "C" fn(Device, *mut PciInfo) -> Return;

struct Api {
    shutdown: Shutdown,
    count: DeviceGetCount,
    handle: DeviceGetHandleByIndex,
    memory: DeviceGetMemoryInfo,
    pci: DeviceGetPciInfo,
    // kept last: dropped after the function pointers above are no longer used
    _library: libloading::Library,
}

impl Api {
    /// Load NVML and initialise it; [`Drop`] shuts it down.
    fn open() -> Result<Self, String> {
        // SAFETY: the symbols have the signatures of NVIDIA's public `nvml.h` (the `_v2`/`_v3`
        // entry points named below). The library is stored in the returned value, so the
        // function pointers never outlive it.
        unsafe {
            let library = libloading::Library::new("libnvidia-ml.so.1").map_err(|e| format!("no NVIDIA NVML library: {e}"))?;
            let init: Init = *library.get(b"nvmlInit_v2\0").map_err(|e| e.to_string())?;
            let shutdown: Shutdown = *library.get(b"nvmlShutdown\0").map_err(|e| e.to_string())?;
            let count: DeviceGetCount = *library.get(b"nvmlDeviceGetCount_v2\0").map_err(|e| e.to_string())?;
            let handle: DeviceGetHandleByIndex = *library.get(b"nvmlDeviceGetHandleByIndex_v2\0").map_err(|e| e.to_string())?;
            let memory: DeviceGetMemoryInfo = *library.get(b"nvmlDeviceGetMemoryInfo\0").map_err(|e| e.to_string())?;
            let pci: DeviceGetPciInfo = *library.get(b"nvmlDeviceGetPciInfo_v3\0").map_err(|e| e.to_string())?;
            // every symbol is there: only now initialise, so `Drop` shuts down only what started
            let code = init();
            if code != SUCCESS {
                return Err(format!("nvmlInit_v2 failed ({code})"));
            }
            Ok(Api { shutdown, count, handle, memory, pci, _library: library })
        }
    }

    fn device_count(&self) -> Result<u32, String> {
        let mut count = 0u32;
        // SAFETY: `count` is a valid, writable u32 for the duration of the call.
        let code = unsafe { (self.count)(&mut count) };
        if code != SUCCESS {
            return Err(format!("nvmlDeviceGetCount_v2 failed ({code})"));
        }
        Ok(count.min(MAX_DEVICES))
    }

    fn reading(&self, index: u32) -> Option<PciMemoryReading> {
        let mut device: Device = std::ptr::null_mut();
        // SAFETY: `device` is a valid out-pointer; NVML writes an opaque handle that stays valid
        // until `nvmlShutdown`, which only runs in `Drop` after every reading.
        if unsafe { (self.handle)(index, &mut device) } != SUCCESS || device.is_null() {
            return None;
        }
        let mut info = PciInfo { bus_id_legacy: [0; 16], domain: 0, bus: 0, device: 0, pci_device_id: 0, pci_sub_system_id: 0, bus_id: [0; 32] };
        // SAFETY: `device` came from NVML above; `info` is a writable `nvmlPciInfo_t`.
        if unsafe { (self.pci)(device, &mut info) } != SUCCESS {
            return None;
        }
        let pci = bus_id(&info.bus_id)?;
        let mut memory = Memory::default();
        // SAFETY: `device` came from NVML above; `memory` is a writable `nvmlMemory_t`.
        let known = unsafe { (self.memory)(device, &mut memory) } == SUCCESS;
        Some(PciMemoryReading {
            pci,
            vram_total_bytes: known.then_some(memory.total),
            // a used value above the total is a broken reading: unknown, not "full"
            vram_used_bytes: (known && memory.used <= memory.total).then_some(memory.used),
        })
    }
}

impl Drop for Api {
    fn drop(&mut self) {
        // SAFETY: NVML was initialised in `open`; every handle has been used by now.
        let _ = unsafe { (self.shutdown)() };
    }
}

/// The PCI address in NVML's NUL-terminated `busId` field.
fn bus_id(raw: &[u8]) -> Option<PciAddress> {
    let end = raw.iter().position(|b| *b == 0).unwrap_or(raw.len());
    PciAddress::parse(std::str::from_utf8(raw.get(..end)?).ok()?)
}

/// Every NVIDIA GPU's VRAM, keyed by PCI address. `Err` when NVML is missing or fails to start;
/// a device that cannot be read is left out.
pub fn readings() -> Result<Vec<PciMemoryReading>, String> {
    let api = Api::open()?;
    let count = api.device_count()?;
    Ok((0..count).filter_map(|i| api.reading(i)).collect())
}

/// NVML readings tied to the render nodes of `inventory`, ready for `Snapshot::apply_gpu_report`.
pub fn reports(inventory: &Inventory) -> Result<Vec<GpuMemoryReport>, String> {
    Ok(cfa4_accel::memory::reports_by_pci(inventory, &readings()?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_c_layouts_match_nvml_h() {
        assert_eq!(std::mem::size_of::<Memory>(), 24);
        assert_eq!(std::mem::size_of::<PciInfo>(), 68);
        assert_eq!(std::mem::offset_of!(PciInfo, bus_id), 36);
    }

    #[test]
    fn bus_ids_parse_or_are_left_out() {
        let mut raw = [0u8; 32];
        raw[..16].copy_from_slice(b"00000000:03:00.0");
        assert_eq!(bus_id(&raw), Some(PciAddress { domain: 0, bus: 3, device: 0, function: 0 }));
        assert_eq!(bus_id(&[0xff; 32]), None, "no NUL, not UTF-8");
        assert_eq!(bus_id(&[0; 32]), None);
    }

    #[test]
    fn a_machine_without_nvidia_gets_an_error_not_a_crash() {
        // on CI and on machines without the driver this is `Err`; with it, a list
        match readings() {
            Ok(list) => assert!(list.len() <= MAX_DEVICES as usize),
            Err(e) => assert!(!e.is_empty()),
        }
    }
}
