# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

<!-- next-version -->

## [[1.20.2](https://github.com/RivoLink/leaf/releases/tag/1.20.2)] - 2026-05-09

### Changed

- resolve cargo-audit warnings ([#112](https://github.com/RivoLink/leaf/pull/112))
- refacto into sub-modules ([#111](https://github.com/RivoLink/leaf/pull/111))

## [[1.20.1](https://github.com/RivoLink/leaf/releases/tag/1.20.1)] - 2026-05-08

### Fixed

- contrib refresh token ([#109](https://github.com/RivoLink/leaf/pull/109))
- pr contrib refresh workflow ([#107](https://github.com/RivoLink/leaf/pull/107))
- contrib refresh workflow ([#106](https://github.com/RivoLink/leaf/pull/106))
- contrib image refresh ([#105](https://github.com/RivoLink/leaf/pull/105))
- toc numkey navigation ([#104](https://github.com/RivoLink/leaf/pull/104))
- redraw on terminal resize ([#102](https://github.com/RivoLink/leaf/pull/102))
- cap max scroll at document end ([#103](https://github.com/RivoLink/leaf/pull/103))

### Changed

- refresh contrib image ([#110](https://github.com/RivoLink/leaf/pull/110))

## [[1.20.0](https://github.com/RivoLink/leaf/releases/tag/1.20.0)] - 2026-05-07

### Added

- clickable links with hover effect ([#96](https://github.com/RivoLink/leaf/pull/96))

### Fixed

- link marker overlap ([#97](https://github.com/RivoLink/leaf/pull/97))

### Changed

- improve search results ux ([#100](https://github.com/RivoLink/leaf/pull/100))

## [[1.19.1](https://github.com/RivoLink/leaf/releases/tag/1.19.1)] - 2026-05-06

### Docs

- windows path tip for config ([#93](https://github.com/RivoLink/leaf/pull/93))
- update contributors image link ([#92](https://github.com/RivoLink/leaf/pull/92))

## [[1.19.0](https://github.com/RivoLink/leaf/releases/tag/1.19.0)] - 2026-05-06

### Docs

- custom theme support ([#83](https://github.com/RivoLink/leaf/pull/83))

### Added

- add custom theme support ([#80](https://github.com/RivoLink/leaf/pull/80))

### Fixed

- verify SHA-256 checksum before installing binary ([#87](https://github.com/RivoLink/leaf/pull/87))
- cargo audit ci install ([#81](https://github.com/RivoLink/leaf/pull/81))

### Changed

- normalize nested code fences ([#86](https://github.com/RivoLink/leaf/pull/86))
- reduce theme RwLock reads ([#82](https://github.com/RivoLink/leaf/pull/82))
- ci workflow and gpg signing ([#78](https://github.com/RivoLink/leaf/pull/78))

## [[1.18.2](https://github.com/RivoLink/leaf/releases/tag/1.18.2)] - 2026-05-03

### Fixed

- link style with modifiers ([#77](https://github.com/RivoLink/leaf/pull/77))
- nested list parent merge ([#75](https://github.com/RivoLink/leaf/pull/75))
- table cell extra-space ([#74](https://github.com/RivoLink/leaf/pull/74))
- watch from config ([#73](https://github.com/RivoLink/leaf/pull/73))

## [[1.18.1](https://github.com/RivoLink/leaf/releases/tag/1.18.1)] - 2026-05-01

### Docs

- add contributing section ([#69](https://github.com/RivoLink/leaf/pull/69))
- add installation instructions for AUR ([#68](https://github.com/RivoLink/leaf/pull/68))

### Fixed

- use tty input reader for piped stdin on macOS ([#67](https://github.com/RivoLink/leaf/pull/67))

### Changed

- update mmdflux version ([#70](https://github.com/RivoLink/leaf/pull/70))

## [[1.18.0](https://github.com/RivoLink/leaf/releases/tag/1.18.0)] - 2026-04-30

### Docs

- config file support ([#65](https://github.com/RivoLink/leaf/pull/65))

### Added

- config file support ([#64](https://github.com/RivoLink/leaf/pull/64))

## [[1.17.0](https://github.com/RivoLink/leaf/releases/tag/1.17.0)] - 2026-04-28

### Docs

- frontmatter yaml support ([#62](https://github.com/RivoLink/leaf/pull/62))

### Added

- frontmatter yaml support ([#61](https://github.com/RivoLink/leaf/pull/61))

## [[1.16.0](https://github.com/RivoLink/leaf/releases/tag/1.16.0)] - 2026-04-28

### Docs

- mermaid render support ([#60](https://github.com/RivoLink/leaf/pull/60))

### Added

- mermaid render support ([#59](https://github.com/RivoLink/leaf/pull/59))

## [[1.15.7](https://github.com/RivoLink/leaf/releases/tag/1.15.7)] - 2026-04-28

### Changed

- improve heading style ([#57](https://github.com/RivoLink/leaf/pull/57))

## [[1.15.6](https://github.com/RivoLink/leaf/releases/tag/1.15.6)] - 2026-04-27

### Fixed

- editor spawn output ([#55](https://github.com/RivoLink/leaf/pull/55))
- table inline styles ([#54](https://github.com/RivoLink/leaf/pull/54))

### Changed

- distinguish heading styles ([#56](https://github.com/RivoLink/leaf/pull/56))

## [[1.15.5](https://github.com/RivoLink/leaf/releases/tag/1.15.5)] - 2026-04-26

### Docs

- add features demo ([#52](https://github.com/RivoLink/leaf/pull/52))

## [[1.15.4](https://github.com/RivoLink/leaf/releases/tag/1.15.4)] - 2026-04-25

### Fixed

- inline code/latex in tables ([#50](https://github.com/RivoLink/leaf/pull/50))

## [[1.15.3](https://github.com/RivoLink/leaf/releases/tag/1.15.3)] - 2026-04-24

### Fixed

- patch rustls-webpki vuln ([#48](https://github.com/RivoLink/leaf/pull/48))

## [[1.15.2](https://github.com/RivoLink/leaf/releases/tag/1.15.2)] - 2026-04-24

### Docs

- clean-up readme sections ([#46](https://github.com/RivoLink/leaf/pull/46))

### Changed

- add jed/xjed editors ([#47](https://github.com/RivoLink/leaf/pull/47))

## [[1.15.1](https://github.com/RivoLink/leaf/releases/tag/1.15.1)] - 2026-04-24

### Docs

- add preview image ([#44](https://github.com/RivoLink/leaf/pull/44))

### Fixed

- windows path prefix ([#43](https://github.com/RivoLink/leaf/pull/43))

### Changed

- detailed help output ([#42](https://github.com/RivoLink/leaf/pull/42))

## [[1.15.0](https://github.com/RivoLink/leaf/releases/tag/1.15.0)] - 2026-04-23

### Added

- latex render support ([#40](https://github.com/RivoLink/leaf/pull/40))

## [[1.14.0](https://github.com/RivoLink/leaf/releases/tag/1.14.0)] - 2026-04-22

### Added

- code line number ([#39](https://github.com/RivoLink/leaf/pull/39))

## [[1.13.0](https://github.com/RivoLink/leaf/releases/tag/1.13.0)] - 2026-04-22

### Added

- path viewer popup ([#37](https://github.com/RivoLink/leaf/pull/37))

### Changed

- refacto modal to popup ([#38](https://github.com/RivoLink/leaf/pull/38))

## [[1.12.0](https://github.com/RivoLink/leaf/releases/tag/1.12.0)] - 2026-04-20

### Added

- interactive drag/hover scrollbar ([#36](https://github.com/RivoLink/leaf/pull/36))

## [[1.11.1](https://github.com/RivoLink/leaf/releases/tag/1.11.1)] - 2026-04-20

### Docs

- update ARCHITECTURE.md ([#34](https://github.com/RivoLink/leaf/pull/34))

### Changed

- mouse selection shortcut ([#35](https://github.com/RivoLink/leaf/pull/35))

## [[1.11.0](https://github.com/RivoLink/leaf/releases/tag/1.11.0)] - 2026-04-19

### Added

- anytime fuzzy/browser picker ([#33](https://github.com/RivoLink/leaf/pull/33))

## [[1.10.2](https://github.com/RivoLink/leaf/releases/tag/1.10.2)] - 2026-04-18

### Docs

- npm packages readme ([#32](https://github.com/RivoLink/leaf/pull/32))

### Fixed

- heading inline code ([#31](https://github.com/RivoLink/leaf/pull/31))

## [[1.10.1](https://github.com/RivoLink/leaf/releases/tag/1.10.1)] - 2026-04-17

### Fixed

- npm publish script ([#30](https://github.com/RivoLink/leaf/pull/30))

## [[1.10.0](https://github.com/RivoLink/leaf/releases/tag/1.10.0)] - 2026-04-17

### Added

- publish npm packages ([#29](https://github.com/RivoLink/leaf/pull/29))

## [[1.9.3](https://github.com/RivoLink/leaf/releases/tag/1.9.3)] - 2026-04-17

### Changed

- add preview image ([#28](https://github.com/RivoLink/leaf/pull/28))

## [[1.9.2](https://github.com/RivoLink/leaf/releases/tag/1.9.2)] - 2026-04-17

### Changed

- auto generate changelog ([#27](https://github.com/RivoLink/leaf/pull/27))

## [[1.9.1](https://github.com/RivoLink/leaf/releases/tag/1.9.1)] - 2026-04-16

### Changed

- improve status-bar shortcuts ([#25](https://github.com/RivoLink/leaf/pull/25))
- improve watch mode ([#26](https://github.com/RivoLink/leaf/pull/26))

## [[1.9.0](https://github.com/RivoLink/leaf/releases/tag/1.9.0)] - 2026-04-16

### Added

- open in editor ([#24](https://github.com/RivoLink/leaf/pull/24))

## [[1.8.0](https://github.com/RivoLink/leaf/releases/tag/1.8.0)] - 2026-04-13

### Changed

- refacto and optimize ([#23](https://github.com/RivoLink/leaf/pull/23))

## [[1.7.0](https://github.com/RivoLink/leaf/releases/tag/1.7.0)] - 2026-04-12

### Changed

- fuzzy picker optimization ([#22](https://github.com/RivoLink/leaf/pull/22))

## [[1.6.1](https://github.com/RivoLink/leaf/releases/tag/1.6.1)] - 2026-04-12

### Fixed

- minor ux bugs ([#21](https://github.com/RivoLink/leaf/pull/21))

## [[1.6.0](https://github.com/RivoLink/leaf/releases/tag/1.6.0)] - 2026-04-11

### Added

- fuzzy markdown picker ([#20](https://github.com/RivoLink/leaf/pull/20))

## [[1.5.1](https://github.com/RivoLink/leaf/releases/tag/1.5.1)] - 2026-04-11

### Docs

- leaf update guides ([#18](https://github.com/RivoLink/leaf/pull/18))
- uninstall guides ([#19](https://github.com/RivoLink/leaf/pull/19))

## [[1.5.0](https://github.com/RivoLink/leaf/releases/tag/1.5.0)] - 2026-04-10

### Added

- auto update ([#17](https://github.com/RivoLink/leaf/pull/17))

## [[1.4.2](https://github.com/RivoLink/leaf/releases/tag/1.4.2)] - 2026-04-10

### Docs

- contributing and security guides ([#16](https://github.com/RivoLink/leaf/pull/16))

### Changed

- improve preview design ([#15](https://github.com/RivoLink/leaf/pull/15))

## [[1.4.1](https://github.com/RivoLink/leaf/releases/tag/1.4.1)] - 2026-04-10

### Fixed

- using static runners version ([#14](https://github.com/RivoLink/leaf/pull/14))

### Changed

- update build runners version ([#12](https://github.com/RivoLink/leaf/pull/12))
- update windows installer ([#13](https://github.com/RivoLink/leaf/pull/13))

## [[1.4.0](https://github.com/RivoLink/leaf/releases/tag/1.4.0)] - 2026-04-10

### Changed

- code review optimization ([#11](https://github.com/RivoLink/leaf/pull/11))

## [[1.3.0](https://github.com/RivoLink/leaf/releases/tag/1.3.0)] - 2026-04-09

### Added

- file picker ([#9](https://github.com/RivoLink/leaf/pull/9))

### Changed

- improve windows installer ([#10](https://github.com/RivoLink/leaf/pull/10))

## [[1.2.0](https://github.com/RivoLink/leaf/releases/tag/1.2.0)] - 2026-04-08

### Fixed

- Cargo.lock version ([#8](https://github.com/RivoLink/leaf/pull/8))

### Changed

- release installers ([#5](https://github.com/RivoLink/leaf/pull/5))
- improve list alignment ([#6](https://github.com/RivoLink/leaf/pull/6))
- add help modal ([#7](https://github.com/RivoLink/leaf/pull/7))

## [[1.1.0](https://github.com/RivoLink/leaf/releases/tag/1.1.0)] - 2026-04-08

### Added

- custom themes ([#1](https://github.com/RivoLink/leaf/pull/1))

### Fixed

- workflow job name ([#4](https://github.com/RivoLink/leaf/pull/4))

### Changed

- improve design ([#2](https://github.com/RivoLink/leaf/pull/2))
- release workflows ([#3](https://github.com/RivoLink/leaf/pull/3))

## [[1.0.0](https://github.com/RivoLink/leaf/releases/tag/1.0.0)] - 2026-04-07

Initial release.
