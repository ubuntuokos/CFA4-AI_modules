# Kodek-backlog (alacsony prioritás)

> **Utolsó frissítés:** 2026-10-27 · **Változás:** major (új fájl: a 2026-10-27-i kodeklistából a még nem megoldott tételek) · **Hely a tervben:** a kodekfejlesztés legvégén, az [M9](roadmap.md#milestones-history-and-remaining) és a [B3](roadmap.md#beta-milestones) után

A tulajdonos 2026-10-27-én két kodeklistát adott át (általánosan használt, valamint ritkább és
ázsiai videó-, audio- és képkodekek). Ebből a fájlból **kimaradt minden, ami már meg van oldva**
(lásd lent), a többi ide került. **Nem sürgős:** ezek a tételek a kodekfejlesztés végére
kerülnek, a meglévő kodekmunka (M9, B3 / [G3](gaps.md#g3-10-bit-and-422-camera-media-needs-hardware),
a [codec-parity.md](codec-parity.md) „Behind” sorai) után. A sávokon belül a sorrend sem kötelező.

Minden tételre érvényesek a szokásos szabályok: clean-room, nyilvános specifikációból, pure Rust
([AGENTS.md](../AGENTS.md) §2); ffmpeg csak tesztorákulum. Ahol nincs nyilvános specifikáció, vagy
szabadalmi / licencdöntés kell, ott **tulajdonosi döntés** szükséges, mielőtt bárki hozzányúl.

## Már megoldva (ezért kimaradt)

A crate-ek README-je, a [codec-parity.md](codec-parity.md) és a [file-format-parity.md](file-format-parity.md) alapján, 2026-10-27-én.

| Terület | Megoldott |
|---|---|
| Videó | H.264 / AVC (8 bit 4:2:0, dekóder + enkóder), HEVC (Main / Main 10), AV1, VP9, VP8, MPEG-2 és MPEG-1, Apple ProRes (mind a hat profil, dekód + enkód), Avid DNxHD / DNxHR, Motion JPEG, Samsung APV (RFC 9924, mind a 7 profil, dekód + enkód; a HQ / SQ / LQ / UQ minőségi szintek enkóderbeállítások) |
| Audio | AAC (LC, HE, LATM), MP3 (dekód), MP1 / MP2, Opus, FLAC (dekód + enkóder), PCM / LPCM, ALAC, Vorbis, AC-3, E-AC-3 (5.1 mag) |
| Kép | JPEG, PNG, WebP, TIFF, BMP, GIF, APNG, OpenEXR (scan-line), DPX, CinemaDNG / DNG, HEIF / HEIC (első kép, 8 bit, külső libheif-programmal), HLG átviteli görbe (ARIB STD-B67, színkezelésben) |
| Konténer | MP4, MOV, MKV / Matroska, WebM, AVI (demux), MPEG-TS / PS, MXF, Ogg, WAV / BWF |

**Nem ide tartozik, mert már szerepel a tervben:** H.264 High 10 / 4:2:2 és HEVC 4:2:2
(B3, G3, #626), E-AC-3 hiányzó részei (#647), MP3 / HEVC / AV1 szoftveres enkóder és
MPEG-2 enkóder ([codec-parity.md](codec-parity.md)). Ezek a helyükön maradnak.

## K1 – elterjedt vagy professzionális (a backlogon belül először)

| Kodek / formátum | Típus | Megjegyzés |
|---|---|---|
| MPEG-4 Part 2 (DivX / Xvid) | videó | Az AVI-demuxer már felismeri, csak dekóder kell |
| JPEG 2000 | videó / kép | MXF, DCP, archiválás |
| DV / DVCPRO / DVCPRO HD | videó | Panasonic P2 is |
| VC-1 / WMV7 / WMV8 / WMV9 | videó | ASF / WMV konténer is kell |
| CineForm | videó | |
| FFV1 | videó | A Matroska-demuxer már ismeri a kodekazonosítót |
| HAP | videó | |
| JPEG XS | videó | |
| Theora | videó | Ma csak az opcionális GStreamer-tartalékon át nyílik meg |
| Tömörítetlen videó (v210, RGB) MOV / AVI-ban | videó | |
| Huffyuv, Ut Video, MagicYUV, Lagarith | videó | Veszteségmentes köztes kodekek |
| Sony XAVC / XAVC S / XAVC HS | kameraprofil | Nagyrészt H.264 / HEVC 10 bit, 4:2:2 – a B3-ra épül; itt csak a metaadat és a mappaszerkezet |
| Canon XF-AVC | kameraprofil | Mint fent |
| Panasonic AVC-Intra / AVC-Ultra / P2 AVC-Intra | kameraprofil | Mint fent, MXF-ben |
| Canon Cinema RAW Light | kamera-RAW | A kamera-RAW csoport része (#383); tulajdonosi döntés |
| Általános kamera-RAW (a DNG-n túl) | kép | #383 |
| DTS, DTS-HD Master Audio | audio | Licenc / szabadalom: tulajdonosi döntés |
| Dolby TrueHD | audio | Licenc / szabadalom: tulajdonosi döntés |
| WMA | audio | |
| ADPCM (IMA, Microsoft) | audio | |
| G.711 / G.722 | audio | |
| AMR-NB / AMR-WB | audio | |
| WavPack | audio | |
| Monkey's Audio (APE) | audio | |
| Speex | audio | |
| AVIF | kép | AV1-dekóder már van, a HEIF-konténer kell hozzá |
| JPEG XL | kép | |
| HEIF / HEIC teljes támogatás | kép | 10 bit / HDR, több kép, animáció; saját dekóder a külső program helyett |
| OpenEXR maradék | kép | PIZ, B44, DWA, csempés, többrészes |
| FLV | konténer | |
| ASF / WMV | konténer | A VC-1 / WMA-hoz |

## K2 – regionális, új generációs, szűkebb szakmai

| Kodek / formátum | Típus | Megjegyzés |
|---|---|---|
| H.266 / VVC | videó | Szabadalmi helyzet: tulajdonosi döntés |
| MPEG-5 EVC | videó | |
| LCEVC | videó | Kiegészítő réteg más kodekekhez |
| AVS1 / AVS-P2, AVS+ | videó | Kínai |
| AVS2 (IEEE 1857.4), AVS3 (IEEE 1857.10) | videó | Kínai UHD / broadcast |
| AVS Audio, AVS2 Audio, AVS3 Audio | audio | Kínai |
| Audio Vivid | audio | Kínai, immerszív; az AVS3 Audiótól külön ellenőrizni |
| DRA (Digital Rise Audio) | audio | Kínai |
| MPEG-H 3D Audio | audio | |
| AC-4 | audio | Licenc: tulajdonosi döntés |
| VC-2, Dirac | videó | |
| Canopus HQ / HQX | videó | |
| Matrox MPEG-2 I-frame | videó | Valószínűleg már a meglévő MPEG-2 dekóderrel megy, csak ellenőrizni kell |
| Sony SR Codec | videó | |
| RealVideo RV10–RV40 | videó | RealMedia konténer is kell |
| RealAudio Cook, SIPR, 14.4 / 28.8 | audio | |
| Sony ATRAC3, ATRAC3plus, ATRAC9 | audio | |
| G.729, EVS, iLBC | audio | Beszédkodekek |
| Musepack SV7 / SV8, TAK, TTA, Shorten, OptimFROG | audio | Ritka veszteségmentes / zenei |

## K3 – örökölt, játék és egyéb ritka (a legvégén)

Ezek többségéhez nincs nyilvános specifikáció, ezért a clean-room szabály miatt mindegyikhez
külön tulajdonosi döntés kell.

| Kodek / formátum | Típus |
|---|---|
| Bink / Bink 2, Bink Audio | játék |
| Smacker, Smacker Audio | játék |
| id RoQ, Interplay MVE, Interplay ACM | játék |
| CRIWARE Sofdec.Prime, Sofdec2, CRI ADX, CRI HCA | japán játék |
| Nintendo THP, Mobiclip, HVQM / HVQM4 | japán játék |
| Nintendo DSP ADPCM, BRSTM, BCSTM / BFSTM | japán játék |
| Sony PSP MPEG-4 AVC profilok | japán játék (valószínűleg a H.264-dekóder elég, csak a konténer kell) |
| Yamaha ADPCM, OKI ADPCM, NTT TwinVQ | régi audio |
| AMV Video | régi kínai lejátszók |
| Cinepak, Intel Indeo 2 / 3 / 4 / 5, Sorenson Video 1 / 3 | régi videó |
| Microsoft RLE / Video 1, Autodesk FLIC / FLC | régi videó |
| HuffYUV MT | régi videó |
| QDesign Music 2, Voxware MetaSound, TrueSpeech, Apple MACE 3 / 6, QCELP | régi audio |
| AVS2 3D / Multiview, AVS3 panoráma / immerszív profilok, AVS3 RTC Speech | kínai speciális |

## Nem kodek, nincs külön feladat

- **ARIB STD-B32** (japán videó / audio / multiplex szabvány): a benne használt kodekek (MPEG-2,
  H.264, HEVC, AAC) és az MPEG-TS már megoldottak; legfeljebb egy ellenőrző teszt kell.
- **ISDB-T, ISDB-S / S3**: rádiós átviteli rendszerek, videószerkesztőben nincs teendő.
- **TPG, SMT (Smart Media Transport), AVS3 Immersive Media**: a K2-es AVS-munka mellett nézendő meg, ha odáig jut.

## Revíziótörténet

| Dátum | Változás | Összefoglaló |
|---|---|---|
| 2026-10-27 | major | Létrehozva a tulajdonos 2026-10-27-i kodeklistájából: a megoldott tételek kiszűrve, a többi három alacsony prioritású sávban a kodekfejlesztés végére |
