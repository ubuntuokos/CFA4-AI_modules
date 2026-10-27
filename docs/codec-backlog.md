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

## Már folyamatban vagy tervezve (ezért kimaradt)

Ezeknek már van helyük a tervben, ezért a duplikáció elkerülése miatt nem kerültek a backlogba.
Ott kell őket követni, nem itt.

| Kodek / formátum | Hol van a tervben |
|---|---|
| H.264 High 10 / 4:2:2, HEVC 4:2:2 (RExt) | B3, [G3](gaps.md#g3-10-bit-and-422-camera-media-needs-hardware), #626 |
| E-AC-3 hiányzó részei (AHT, enhanced coupling, 7.1) | [G10](gaps.md#g10-container-and-codec-gaps-on-import), #647 |
| MPEG-4 Part 2 (DivX / Xvid) | G10, [codec-parity.md](codec-parity.md) (10–20 h) |
| Theora, FLV | GStreamer-tartalék, ADR 0002 2. lépés (kész, de csak fejlesztői buildben) |
| JPEG 2000, JPEG XS | G10, G13 (JPEG 2000 MXF, DCP), [ADR 0003](adr/ADR-0003-aswf-and-conversion-sources.md) A3 |
| Kamera-RAW (Canon Cinema RAW Light és a többi) | G10, #383, #345, ADR 0003 A3 (LibRaw) |
| MP3, HEVC, AV1 szoftveres enkóder, MPEG-2 enkóder | G13, ADR 0003 A5 |

## K1 – elterjedt vagy professzionális (a backlogon belül először)

| Kodek / formátum | Típus | Megjegyzés |
|---|---|---|
| CineForm | videó | |
| FFV1 | videó | A Matroska-demuxer már ismeri a kodekazonosítót |
| HAP | videó | |
| Huffyuv, Ut Video, MagicYUV, Lagarith | videó | Veszteségmentes köztes kodekek |
| DTS, DTS-HD Master Audio | audio | Licenc / szabadalom: tulajdonosi döntés |
| Dolby TrueHD | audio | Licenc / szabadalom: tulajdonosi döntés |
| ADPCM (IMA, Microsoft) | audio | |
| G.711 / G.722 | audio | |
| AMR-NB / AMR-WB | audio | |
| WavPack | audio | |
| Monkey's Audio (APE) | audio | |
| Speex | audio | |
| AVIF | kép | AV1-dekóder már van, a HEIF-konténer kell hozzá |
| JPEG XL | kép | |

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
| 2026-10-27 | minor | A már folyamatban lévő vagy tervezett tételek (G3, G10, G13, ADR 0002, ADR 0003) is kikerültek, külön táblában hivatkozva |
| 2026-10-27 | major | Létrehozva a tulajdonos 2026-10-27-i kodeklistájából: a megoldott tételek kiszűrve, a többi három alacsony prioritású sávban a kodekfejlesztés végére |
