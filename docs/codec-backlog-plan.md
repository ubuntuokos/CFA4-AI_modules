# Kodek-backlog végrehajtási terv

> **Utolsó frissítés:** 2026-10-27 · **Változás:** major (új fájl) · **Forrás:** [codec-backlog.md](codec-backlog.md)

A [codec-backlog.md](codec-backlog.md) 89 tétele heti ütemezésben. **Követelmény: minden héten
legalább 2 tétel elkészül** (a tulajdonos döntése, 2026-10-27). Így a backlog 45 hét alatt fogy el:
**2026-11-02-től 2027-09-12-ig**. Ha egy héten több is elkészül, a következő tételek előrébb jönnek.

Az ütemterv a meglévő kodekmunkával (M9, B3) **párhuzamosan, alacsony prioritással** fut. Ha a
backlognak meg kell várnia az M9 és a B3 lezárását, minden dátum ugyanannyi héttel tolódik.

## Mikor „kész” egy tétel

1. Clean-room dekóder nyilvános specifikációból, pure Rust, `unsafe` nélkül ([AGENTS.md](../AGENTS.md) §0, §2), új L0 crate-ben vagy a meglévőben.
2. Be van kötve az importba (a megfelelő konténerben: AVI, MOV, MKV, MP4, Ogg, nyers fájl).
3. ffmpeg-orákulumteszt (bitpontos, vagy veszteséges hangnál SNR-határ), generált fixture-ökkel a `target/fixtures/` alatt.
4. Crate-README, valamint egy sor a [codec-parity.md](codec-parity.md)-ben; a tétel kikerül a [codec-backlog.md](codec-backlog.md)-ből.
5. `cargo xtask ci` zöld; egy tétel = egy commit (`CB-<hét>.<1|2>: <kodek>`).

Az enkódolás nem része a tételnek, csak ha a sor külön jelzi.

## Döntési kapuk

| Jel | Jelentés | Határidő |
|---|---|---|
| ⚖️ | Licenc / szabadalom (Dolby, DTS, VVC, MPEG-H, AC-4): tulajdonosi döntés kell | A K1-es tételekre a 7. hétig (2026-12-14), a K2-esekre a 24. hétig (2027-04-12) |
| 📄 | Nincs teljes nyilvános specifikáció, a clean-room szabály miatt tulajdonosi döntés kell | A K2-esekre a 12. hétig (2027-01-18), a K3-asokra a 25. hétig (2027-04-19) |

Ha egy tételre a döntés nemleges vagy késik, az a hét a következő szabad tétellel telik meg, így a
heti 2 tétel nem csúszik. A nemleges döntésű tételek kikerülnek a backlogból.

## Ütemterv

Sávok: K1 az 1–9. hét, K2 a 10–26. hét, K3 a 26–43. hét, K4 a 44–45. hét.

| Hét | Kezdete | Sáv | 1. tétel | 2. tétel |
|---|---|---|---|---|
| 1 | 2026-11-02 | K1 | FFV1 | Huffyuv |
| 2 | 2026-11-09 | K1 | Ut Video | MagicYUV |
| 3 | 2026-11-16 | K1 | Lagarith | HAP |
| 4 | 2026-11-23 | K1 | CineForm | G.711 / G.722 |
| 5 | 2026-11-30 | K1 | ADPCM (IMA, Microsoft) | WavPack |
| 6 | 2026-12-07 | K1 | Monkey's Audio (APE) | Speex |
| 7 | 2026-12-14 | K1 | AMR-NB / AMR-WB | JPEG XL |
| 8 | 2026-12-21 | K1 | AVIF | DTS (core) ⚖️ |
| 9 | 2026-12-28 | K1 | DTS-HD Master Audio ⚖️ | Dolby TrueHD ⚖️ |
| 10 | 2027-01-04 | K2 | Matrox MPEG-2 I-frame (ellenőrzés + teszt) | VC-2 |
| 11 | 2027-01-11 | K2 | Dirac | Canopus HQ / HQX |
| 12 | 2027-01-18 | K2 | TTA | Shorten |
| 13 | 2027-01-25 | K2 | TAK 📄 | Musepack SV7 / SV8 |
| 14 | 2027-02-01 | K2 | OptimFROG 📄 | G.729 |
| 15 | 2027-02-08 | K2 | iLBC | EVS |
| 16 | 2027-02-15 | K2 | RealVideo RV10 / RV20 📄 | RealVideo RV30 / RV40 📄 |
| 17 | 2027-02-22 | K2 | RealAudio Cook 📄 | RealAudio SIPR, 14.4 / 28.8 📄 |
| 18 | 2027-03-01 | K2 | ATRAC3 📄 | ATRAC3plus 📄 |
| 19 | 2027-03-08 | K2 | ATRAC9 📄 | Sony SR Codec 📄 |
| 20 | 2027-03-15 | K2 | MPEG-5 EVC | LCEVC |
| 21 | 2027-03-22 | K2 | AVS1 / AVS+ | AVS2 |
| 22 | 2027-03-29 | K2 | AVS3 | AVS Audio |
| 23 | 2027-04-05 | K2 | AVS2 Audio | AVS3 Audio |
| 24 | 2027-04-12 | K2 | Audio Vivid | DRA |
| 25 | 2027-04-19 | K2 | H.266 / VVC ⚖️ | MPEG-H 3D Audio ⚖️ |
| 26 | 2027-04-26 | K2 / K3 | AC-4 ⚖️ | Bink Video 📄 |
| 27 | 2027-05-03 | K3 | Bink 2 📄 | Bink Audio 📄 |
| 28 | 2027-05-10 | K3 | Smacker + Smacker Audio 📄 | id RoQ 📄 |
| 29 | 2027-05-17 | K3 | Interplay MVE + ACM 📄 | CRI ADX 📄 |
| 30 | 2027-05-24 | K3 | CRI HCA 📄 | Sofdec.Prime 📄 |
| 31 | 2027-05-31 | K3 | Sofdec2 (konténer) 📄 | Nintendo THP 📄 |
| 32 | 2027-06-07 | K3 | Nintendo Mobiclip 📄 | Nintendo HVQM / HVQM4 📄 |
| 33 | 2027-06-14 | K3 | Nintendo DSP ADPCM + BRSTM 📄 | BCSTM / BFSTM 📄 |
| 34 | 2027-06-21 | K3 | PSP MPEG-4 AVC (PMF) 📄 | Yamaha + OKI ADPCM 📄 |
| 35 | 2027-06-28 | K3 | NTT TwinVQ 📄 | AMV Video 📄 |
| 36 | 2027-07-05 | K3 | Cinepak 📄 | Intel Indeo 2 / 3 📄 |
| 37 | 2027-07-12 | K3 | Intel Indeo 4 / 5 📄 | Sorenson Video 1 📄 |
| 38 | 2027-07-19 | K3 | Sorenson Video 3 📄 | Microsoft RLE + Video 1 📄 |
| 39 | 2027-07-26 | K3 | Autodesk FLIC / FLC | HuffYUV MT |
| 40 | 2027-08-02 | K3 | QDesign Music 2 📄 | Voxware MetaSound 📄 |
| 41 | 2027-08-09 | K3 | TrueSpeech 📄 | Apple MACE 3 / 6 📄 |
| 42 | 2027-08-16 | K3 | QCELP | AVS2 3D / Multiview |
| 43 | 2027-08-23 | K3 | AVS3 panoráma / immerszív profilok | AVS3 RTC Speech |
| 44 | 2027-08-30 | K4 | TPG | SMT (Smart Media Transport) |
| 45 | 2027-09-06 | K4 | AVS3 Immersive Media | — |

## Követés

Minden hét végén a hét két tételét pipálni kell ebben a fájlban (✅ a tétel neve elé), és ha
valamelyik nem készült el, a következő hétre kell vinni a pótlással együtt.

## Revíziótörténet

| Dátum | Változás | Összefoglaló |
|---|---|---|
| 2026-10-27 | major | Létrehozva: 89 tétel, heti 2, 45 hét (2026-11-02 – 2027-09-12), döntési kapukkal |
