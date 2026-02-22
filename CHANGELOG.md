## [1.0.4](https://github.com/WebProject-xyz/php-version-manager/compare/v1.0.3...v1.0.4) (2026-02-22)

### Bug Fixes

* add Renovate configuration file ([cb24073](https://github.com/WebProject-xyz/php-version-manager/commit/cb24073f88119813a81c703ec63215dcb1e55486))

## [1.0.3](https://github.com/WebProject-xyz/php-version-manager/compare/v1.0.2...v1.0.3) (2026-02-21)

### Bug Fixes

* remove cargo check and allow rust to publish with dirty lockfile ([fefddcd](https://github.com/WebProject-xyz/php-version-manager/commit/fefddcd5f4a43ea95ea9283c684a68d2bf042b3d))
* skip dev profile verification during cargo publish and sync lockfile ([c67edb5](https://github.com/WebProject-xyz/php-version-manager/commit/c67edb57cf24cef830fbd2778d297e6a54edcb4d))

## [1.0.2](https://github.com/WebProject-xyz/php-version-manager/compare/v1.0.1...v1.0.2) (2026-02-21)

### Bug Fixes

* update Cargo.lock and allow-dirty to bypass publish error ([be15c3a](https://github.com/WebProject-xyz/php-version-manager/commit/be15c3a762867c57c7a046bedda86cd2f7f0515f))
* update Cargo.lock during semantic-release prepare phase to avoid publish errors ([a92c3de](https://github.com/WebProject-xyz/php-version-manager/commit/a92c3de5e5a74dcb80bb3313f1ed83d19905c99d))

## [1.0.1](https://github.com/WebProject-xyz/php-version-manager/compare/v1.0.0...v1.0.1) (2026-02-21)

### Bug Fixes

* pass github app token to action-gh-release to avoid permission error ([5461490](https://github.com/WebProject-xyz/php-version-manager/commit/546149064c52b2740257607284ab047536bac45c))
* re-architect release pipeline to ensure version correctness ([8636f04](https://github.com/WebProject-xyz/php-version-manager/commit/8636f04932334de987dda03ee9745caf25dd41ad))

## 1.0.0 (2026-02-21)

### Features

* initial commit for pvm ([15ce607](https://github.com/WebProject-xyz/php-version-manager/commit/15ce607227eeb046eeb3fa275221ce4df212342b))

### Bug Fixes

* use GitHub App token for semantic-release ([b8e2354](https://github.com/WebProject-xyz/php-version-manager/commit/b8e235425c8ba2aba1f2bf76d4d82b3d313a679f))
* use macos-latest for x86_64 target to fix unsupported runner error ([98cd2af](https://github.com/WebProject-xyz/php-version-manager/commit/98cd2af7378186cdea35ee904db1c685fc269caf))
