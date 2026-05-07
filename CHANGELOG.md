## [1.2.0](https://github.com/WebProject-xyz/php-version-manager/compare/v1.1.2...v1.2.0) (2026-05-07)

### Features

* **self-update:** add self-update command for in-place pvm upgrades ([#19](https://github.com/WebProject-xyz/php-version-manager/issues/19)) ([d77f0ef](https://github.com/WebProject-xyz/php-version-manager/commit/d77f0efd9d7aa8ef6ed5ca55a3d4524c4ab1ab7c))

## [1.1.2](https://github.com/WebProject-xyz/php-version-manager/compare/v1.1.1...v1.1.2) (2026-05-06)

### Bug Fixes

* **ci:** pass App token via 'token' input to action-gh-release ([#22](https://github.com/WebProject-xyz/php-version-manager/issues/22)) ([abeeac3](https://github.com/WebProject-xyz/php-version-manager/commit/abeeac3aa6e53cd35c34f20957a9f6b91e232d67)), closes [softprops/action-gh-release#751](https://github.com/softprops/action-gh-release/issues/751) [#20](https://github.com/WebProject-xyz/php-version-manager/issues/20)

## [1.1.1](https://github.com/WebProject-xyz/php-version-manager/compare/v1.1.0...v1.1.1) (2026-05-06)

### Bug Fixes

* **ci:** pin build matrix toolchain to 1.95.0 ([4b8fbdd](https://github.com/WebProject-xyz/php-version-manager/commit/4b8fbdd2faa9cb33e2bcd200ad74c63eb35a1c66)), closes [#20](https://github.com/WebProject-xyz/php-version-manager/issues/20)

## [1.1.0](https://github.com/WebProject-xyz/php-version-manager/compare/v1.0.4...v1.1.0) (2026-04-29)

### Features

* improve concurrency safety, cross-platform support and dynamic versioning ([85e6c80](https://github.com/WebProject-xyz/php-version-manager/commit/85e6c806e3abc5b8e2c011a48bb1a36b8dac4614))
* improve concurrency safety, cross-platform support and dynamic versioning ([3bc8179](https://github.com/WebProject-xyz/php-version-manager/commit/3bc8179a1323e42add24f9c113ca7d2d086df25a))
* support multiple PHP packages via new bulk API ([5f20206](https://github.com/WebProject-xyz/php-version-manager/commit/5f202060d39a4ec88e2e07828cae75a8c484bfd2))

### Bug Fixes

* address CodeRabbit review findings ([05f9621](https://github.com/WebProject-xyz/php-version-manager/commit/05f9621d423dcd4cfe6458f9132c18efcb3256e4))
* address remaining CodeRabbit findings on PR [#6](https://github.com/WebProject-xyz/php-version-manager/issues/6) ([e6130ed](https://github.com/WebProject-xyz/php-version-manager/commit/e6130ed1fd30f61b84ac179523e98582ddd5b4ad))
* address second-round CodeRabbit review on PR [#6](https://github.com/WebProject-xyz/php-version-manager/issues/6) ([0f9c7f0](https://github.com/WebProject-xyz/php-version-manager/commit/0f9c7f06e46018cce3c5c441b8b696211cff480e))
* resolve clippy::collapsible-if lint in network.rs ([4923dc1](https://github.com/WebProject-xyz/php-version-manager/commit/4923dc12a76dfb0592f0e18893973d053be3ab8c))
* **shell:** add RANDOM entropy to env_file names ([19efe4c](https://github.com/WebProject-xyz/php-version-manager/commit/19efe4cb0444b52dd7476d3c8c9f1eb4b32567bc))

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
