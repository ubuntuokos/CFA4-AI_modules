# CFA4 licensing policy

## Összefoglaló (magyarul)

- A CFA4 licence **GPL-3.0-or-later** (GNU GPL 3. vagy későbbi változat). A teljes szöveg a
  [`LICENSE`](../LICENSE) fájlban van.
- Az upstream projektből (lásd `NOTICE`) örökölt kód és eszközök az eredeti **MIT OR Apache-2.0** feltételek alatt is
  elérhetők maradnak; a `LICENSE-MIT`, `LICENSE-APACHE` és `NOTICE` fájlokat meg kell tartani.
- **Átvehető kód:** bármely GPL-3.0-kompatibilis licencű projektből (GPL-2.0-or-later, GPL-3.0,
  LGPL-2.1-or-later, LGPL-3.0, MIT, BSD, Apache-2.0, MPL-2.0 …), forrásmegjelöléssel és az eredeti
  szerzői jogi fejlécek megtartásával. Minden átvételt be kell írni a lenti nyilvántartásba.
- **Nem vehető át:** GPL-2.0-only kód (pl. HandBrake), AGPL, nem szabad licencű vagy ismeretlen
  licencű kód, és bármely zárt program (Adobe Premiere, DaVinci Resolve) kódja vagy eszközei.
  Ezeknek a képességét saját kóddal vagy külső programként hívva kell megvalósítani.
- **Nincs fizetős rész** (a tulajdonos döntése, 2026-11-08; felváltja a korábbi „ingyenes alap,
  fizetős kiegészítők” modellt): a CFA4 minden része ingyenes és szabadon terjeszthető. Nincs
  fizetős kiegészítő vagy worker, fizetős LTS-ág vagy csomag, licenckulcs, vízjel, próbaidő vagy
  használati korlát, és a CFA4-nek nincs kereskedelmi része. A fejlesztés **önkéntes adománnyal**
  támogatható; az adomány semmilyen funkciót nem nyit meg. A CFA4 folyamatába betöltődő zárt
  bővítmény nem megengedett. Részletek: 4. pont.

*Ez a dokumentum projektszabályzat, nem jogi tanácsadás. Kiadás előtt érdemes jogásszal
átnézetni.*

---

## 1. Project licence

CFA4 is distributed under the **GNU General Public License, version 3 or (at your option) any
later version** (SPDX: `GPL-3.0-or-later`). The full text is in [`LICENSE`](../LICENSE).

- New source files carry the header (or at least the SPDX line):

  ```rust
  // SPDX-License-Identifier: GPL-3.0-or-later
  // Copyright (C) <year> <author> and the CFA4 contributors
  ```

- Contributions are accepted under `GPL-3.0-or-later` (inbound = outbound). No contributor licence
  agreement (CLA) is required at the moment; if one is introduced later it applies only to new
  contributions.

## 2. Inherited upstream code

CFA4 derives from an upstream open-source video editor (its copyright notice is kept verbatim in
[`NOTICE`](../NOTICE)), which is licensed `MIT OR Apache-2.0`. Both licences allow
the code to be combined into a GPL-3.0-or-later work. The obligations that remain:

- keep `LICENSE-MIT`, `LICENSE-APACHE` and the upstream notice in [`NOTICE`](../NOTICE), in the
  repository and in every package (`packaging/env.sh` `copy_docs` ships them);
- keep existing copyright lines in inherited files; do not replace them, add to them;
- existing `*.attribution` sidecars keep the licence the asset was published under.

The upstream name, wordmark and logos are trademarks, not covered by any of these licences; they
have been removed from CFA4 and must not be reintroduced (see `AGENTS.md`).

## 3. What code may be reused

Allowed, with provenance (section 5):

| Source licence | Reuse in CFA4 | Notes |
|---|---|---|
| GPL-3.0-only, GPL-3.0-or-later | yes | e.g. OpenShot, Subtitle Edit, Krita, Audacity 3.x |
| GPL-2.0-or-later | yes (used under GPL-3.0) | e.g. most of Kdenlive/KDE; check every file header |
| LGPL-2.1-or-later, LGPL-3.0 | yes | linking or copying both fine inside a GPL work |
| MIT, BSD-2/3-Clause, ISC, Zlib, 0BSD, BSL-1.0, Unicode, Unlicense, CC0 | yes | keep the notice |
| Apache-2.0 | yes | GPL-3 compatible; keep NOTICE text if the source has one |
| MPL-2.0 | yes | MPL files stay MPL; modifications to those files are published |

Not allowed:

| Source | Why | Alternative |
|---|---|---|
| GPL-2.0-only (e.g. HandBrake, parts of the Linux kernel) | incompatible with GPL-3.0 | call the program as an external process, or reimplement |
| AGPL-3.0 | network-use obligations we do not want on the whole product | reimplement |
| GPL code linked with GPL-incompatible libraries | cannot be distributed | — |
| Proprietary software (Adobe Premiere, DaVinci Resolve, …) | no licence | reimplement the behaviour from public documentation and observation; never copy code, icons, presets, LUTs or fonts |
| Unknown / no licence | all rights reserved | ask the author or leave it out |

A project's overall licence is not enough: **check the header of every file you take**, because
large projects mix licences (some Kdenlive or Krita files are GPL-2.0-only, some FFmpeg parts are
GPL while the default build is LGPL).

External programs (ffmpeg, HandBrakeCLI, …) started as separate processes and communicating
through files, pipes or command-line arguments are not part of CFA4's code and do not change its
licence. If such a program is bundled in a package, its own licence text and source offer must
ship with it.

## 4. Free of charge, supported by donations

**Owner decision, 2026-11-08** (replaces the earlier "free core, paid extras" model): CFA4 has no
paid or commercial part. Everything the project makes is free software under `GPL-3.0-or-later`
and free of charge:

- no paid extras, editions, add-ons or paid AI workers; no licence keys, trial periods, usage
  limits, resolution caps or watermarks anywhere (core, CFA4 AI workers, packages);
- no paid long-term-support branches, paid packages or paid services; LTS branches, packages and
  updates, when they exist, are free for everyone;
- non-code content made by the project (templates, presets, sample media) is openly licensed
  like every other asset (`AGENTS.md` §1).

**Voluntary support:** anyone who wants to may support development with a donation. A donation
never unlocks a feature, a build or priority support, and the program never nags for one.

**Not allowed:** proprietary plug-ins loaded into the CFA4 process (dynamic libraries, Rust
crates, Python modules). Such a plug-in forms a combined work with the GPL core and would have to
be GPL as well. Third parties may write their own workers against the permissively licensed
[CFA4-sdk](https://github.com/ubuntuokos/CFA4-sdk) protocol, but the CFA4 project itself ships
no proprietary or paid worker.

## 5. Reused code register

Every piece of code taken from another project is listed here (one row per source, file or
directory). The copied files keep their original copyright and licence headers, and the commit
that adds them names the source and revision.

| CFA4 path | Origin (project, URL, revision, path) | Copyright holders | Licence |
|---|---|---|---|
| *(none yet)* | | | |

## 6. Dependency checks

- Rust dependencies: `deny.toml` lists the licences that are GPL-3.0 compatible. Run
  `cargo deny check licenses` (installed with `cargo install cargo-deny --locked`); the manual full
  validation in CI runs it too. Dual-licensed crates (`MIT OR Apache-2.0 OR LGPL`, …) pass as long
  as one option is allowed.
- Python: `PySide6`/Qt 6 (LGPL-3.0) is compatible. GPL-only Qt modules are compatible with CFA4's
  licence as well, but must be listed in `NOTICE` when shipped.
- Packages must ship `LICENSE`, `LICENSE-MIT`, `LICENSE-APACHE`, `NOTICE` and the licences of
  bundled fonts and programs.
