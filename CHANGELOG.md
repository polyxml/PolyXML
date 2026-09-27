# [0.27.0](https://github.com/polyxml/PolyXML/compare/v0.26.0...v0.27.0) (2026-09-27)


### Features

* validate Gregorian partial date types across targets ([#87](https://github.com/polyxml/PolyXML/issues/87)) ([cefdeb8](https://github.com/polyxml/PolyXML/commit/cefdeb8f53b62387d914faac1857499f3b98572f))

# [0.26.0](https://github.com/polyxml/PolyXML/compare/v0.25.0...v0.26.0) (2026-09-27)


### Bug Fixes

* handle xsi:nil before abstract type dispatch ([#86](https://github.com/polyxml/PolyXML/issues/86)) ([31c9c83](https://github.com/polyxml/PolyXML/commit/31c9c83da4f30aacf0f929c71cb87dc2b8aba8c6))


### Features

* **java:** emit opt-in Jakarta validation annotations ([#84](https://github.com/polyxml/PolyXML/issues/84)) ([7cc2ab8](https://github.com/polyxml/PolyXML/commit/7cc2ab80f271fd67389cb02e54aabb56f29b3044))

# [0.25.0](https://github.com/polyxml/PolyXML/compare/v0.24.2...v0.25.0) (2026-09-27)


### Features

* **codegen:** preserve document order in unbounded xs:choice ([#79](https://github.com/polyxml/PolyXML/issues/79)) ([19ed893](https://github.com/polyxml/PolyXML/commit/19ed893594f98274ce64a5b7a393cd05bc4cc063))

## [0.24.2](https://github.com/polyxml/PolyXML/compare/v0.24.1...v0.24.2) (2026-09-27)


### Bug Fixes

* **codegen:** harden identifier sanitization, member collisions, and enum symbols ([#82](https://github.com/polyxml/PolyXML/issues/82)) ([c54aa63](https://github.com/polyxml/PolyXML/commit/c54aa635c359c9e99ea1579b5f908530fd307d9c))

## [0.24.1](https://github.com/polyxml/PolyXML/compare/v0.24.0...v0.24.1) (2026-09-27)


### Bug Fixes

* **go:** synchronize polyxml.h with crates/polyxml-c/include/polyxml.h for CI check ([7f1fe9a](https://github.com/polyxml/PolyXML/commit/7f1fe9a21d218c8bf6486e080d6482b1d4085ba5))

# [0.24.0](https://github.com/polyxml/PolyXML/compare/v0.23.3...v0.24.0) (2026-09-26)


### Features

* **python:** add AOT PyO3 native extension codegen backend (--backend aot) ([cc3e89d](https://github.com/polyxml/PolyXML/commit/cc3e89d2fd26adc67614c96ff0f5923173cd72fa)), closes [#48](https://github.com/polyxml/PolyXML/issues/48)

## [0.23.3](https://github.com/polyxml/PolyXML/compare/v0.23.2...v0.23.3) (2026-09-26)


### Bug Fixes

* **ci:** eliminate .NET first-time migration race condition in tests ([3d21af3](https://github.com/polyxml/PolyXML/commit/3d21af3757d57510d9abda240109635e196d378e))

## [0.23.2](https://github.com/polyxml/PolyXML/compare/v0.23.1...v0.23.2) (2026-09-26)

### Packages

* Published [`@polyxml/wasm`](https://www.npmjs.com/package/@polyxml/wasm) to npm for browsers, Node.js, and Bun.


### Bug Fixes

* **go:** migrate module to polyxml organization ([3a18d99](https://github.com/polyxml/PolyXML/commit/3a18d998108eaf54f67dd04c182d5c144c93c6be))

## [0.23.1](https://github.com/polyxml/PolyXML/compare/v0.23.0...v0.23.1) (2026-09-26)


### Bug Fixes

* **release:** fail crates publish on real errors ([bfa5e65](https://github.com/polyxml/PolyXML/commit/bfa5e658445a76f13a1b416e1f4089fb6a303e53))

# [0.23.0](https://github.com/nth-bailey/PolyXML/compare/v0.22.0...v0.23.0) (2026-09-23)


### Features

* **wasm:** stream XML records and benchmark runtimes ([#70](https://github.com/nth-bailey/PolyXML/issues/70)) ([5cf91a9](https://github.com/nth-bailey/PolyXML/commit/5cf91a957882852afb4da795ab93e44371226ad7))

# [0.22.0](https://github.com/nth-bailey/PolyXML/compare/v0.21.0...v0.22.0) (2026-09-23)


### Features

* **wasm:** add XML and JSON runtime package ([#69](https://github.com/nth-bailey/PolyXML/issues/69)) ([43b6794](https://github.com/nth-bailey/PolyXML/commit/43b6794e4d4159698b460ffad049ebc23ecad485))

# [0.21.0](https://github.com/nth-bailey/PolyXML/compare/v0.20.1...v0.21.0) (2026-09-23)


### Bug Fixes

* enforce pattern and xsi:type semantics across runtimes ([d6f567d](https://github.com/nth-bailey/PolyXML/commit/d6f567d148d4f0ca46786fdf755f580bd72b4f54))
* **java-codegen:** inline base fields in plain record mode ([5e559ee](https://github.com/nth-bailey/PolyXML/commit/5e559ee8d11bbe831ec5baa4651dec91993d938b))
* **rust-codegen:** inline xsd:extension base fields in generated structs ([93713b3](https://github.com/nth-bailey/PolyXML/commit/93713b353a41a50e02b9686072e896badb19bb68))
* simpleContent text codecs, split-text accumulation, and transcoder entity refs ([c16e57d](https://github.com/nth-bailey/PolyXML/commit/c16e57d30a1501a154e6d02c7dfc921b4b16be8e)), closes [#51](https://github.com/nth-bailey/PolyXML/issues/51)


### Features

* **cli:** unify generation options and add target-aware completion ([a5337af](https://github.com/nth-bailey/PolyXML/commit/a5337af3cf25569dbb2b059eb9724c8f7ca90c05)), closes [#50](https://github.com/nth-bailey/PolyXML/issues/50)
* **rust-codegen:** perfect-hash element tag dispatch via --feature phf ([dce03c1](https://github.com/nth-bailey/PolyXML/commit/dce03c1c1633e22c73821e0e7cbd1a728922b97a)), closes [#49](https://github.com/nth-bailey/PolyXML/issues/49)
* **schema:** preserve xs:pattern OR/AND semantics and enforce patterns in all 7 codegens (issue [#54](https://github.com/nth-bailey/PolyXML/issues/54)) ([05466ba](https://github.com/nth-bailey/PolyXML/commit/05466ba903f3139162619bf2d76560c8811399e3))
* **schema:** xsi:type polymorphic dispatch for abstract complexTypes (issue [#53](https://github.com/nth-bailey/PolyXML/issues/53)) ([2c6e611](https://github.com/nth-bailey/PolyXML/commit/2c6e6119d703d1c0c5d03a2652938ff938b6ffb0))

## [0.20.1](https://github.com/nth-bailey/PolyXML/compare/v0.20.0...v0.20.1) (2026-09-22)


### Bug Fixes

* harden edge-case XSD handling in parser and all 7 codegens (issue [#51](https://github.com/nth-bailey/PolyXML/issues/51)) ([e7920a2](https://github.com/nth-bailey/PolyXML/commit/e7920a2e1a4e052e35f4387ef680559f8cf41e8e))

# [0.20.0](https://github.com/nth-bailey/PolyXML/compare/v0.19.2...v0.20.0) (2026-09-22)


### Bug Fixes

* **java:** cache direct-codec StAX factories per thread ([3eecb74](https://github.com/nth-bailey/PolyXML/commit/3eecb7485a64f749faecc79fbecc610e6a92698e)), closes [#46](https://github.com/nth-bailey/PolyXML/issues/46)


### Features

* **codegen:** add mutable model styles, builders, and direct Java codecs ([65e86a7](https://github.com/nth-bailey/PolyXML/commit/65e86a74e087243c28b7fe67291c1b73f1ceeb27))

## [0.19.2](https://github.com/nth-bailey/PolyXML/compare/v0.19.1...v0.19.2) (2026-09-20)


### Performance Improvements

* **core,python:** optimize parser, serializer, and dataclass vectorcall ([5674d7e](https://github.com/nth-bailey/PolyXML/commit/5674d7ec33ccd95bcb12bed9f74fac193f308c22))

## [0.19.1](https://github.com/nth-bailey/PolyXML/compare/v0.19.0...v0.19.1) (2026-09-20)


### Bug Fixes

* **clippy:** use char push in python custom_header ([dd15487](https://github.com/nth-bailey/PolyXML/commit/dd1548743e0dc7554a758a40d77ac740d2b63119))
* **csharp:** use generic JsonStringEnumConverter and typed default for single-property constructors ([aea99e6](https://github.com/nth-bailey/PolyXML/commit/aea99e6fca429884ae651281cc08e7a89f9a88c9))
* **python:** automatically adapt C-style comments (//) to (#) in custom_header ([6eada7a](https://github.com/nth-bailey/PolyXML/commit/6eada7ab730f811688b158de0dc7138b3db1ebd7))

# [0.19.0](https://github.com/nth-bailey/PolyXML/compare/v0.18.0...v0.19.0) (2026-09-20)


### Features

* **codegen:** add [@generated](https://github.com/generated) tag and custom_header configuration ([a939577](https://github.com/nth-bailey/PolyXML/commit/a93957760e1196a85329d0742105b31835611405))

# [0.18.0](https://github.com/nth-bailey/PolyXML/compare/v0.17.0...v0.18.0) (2026-09-20)


### Features

* **dist:** add universal install script, deb/rpm packaging, and release-binaries CI workflow ([e00f307](https://github.com/nth-bailey/PolyXML/commit/e00f3073db867a7726d7addfb77df38bddc2b0b5))

# [0.17.0](https://github.com/nth-bailey/PolyXML/compare/v0.16.0...v0.17.0) (2026-09-20)


### Features

* **codegen:** support C# source-gen, TS Valibot/TypeBox, Go Sonic/EasyJSON, and Rust rkyv (closes [#40](https://github.com/nth-bailey/PolyXML/issues/40), [#42](https://github.com/nth-bailey/PolyXML/issues/42), [#43](https://github.com/nth-bailey/PolyXML/issues/43), [#45](https://github.com/nth-bailey/PolyXML/issues/45)) ([ba410bc](https://github.com/nth-bailey/PolyXML/commit/ba410bc2ce8bde5e36f13304556a3af316949c2d))

# [0.16.0](https://github.com/nth-bailey/PolyXML/compare/v0.15.0...v0.16.0) (2026-09-20)


### Features

* **codegen/cpp:** add C++20 Modules (--mode modules) and Glaze compile-time serde (--backend glaze) ([8c30bd3](https://github.com/nth-bailey/PolyXML/commit/8c30bd33ca71f3f46730502498f0fc9248124ae6)), closes [#44](https://github.com/nth-bailey/PolyXML/issues/44)

# [0.15.0](https://github.com/nth-bailey/PolyXML/compare/v0.14.3...v0.15.0) (2026-09-20)


### Features

* **codegen/java:** add Jackson backend support for Spring Boot / enterprise serialization ([78fe976](https://github.com/nth-bailey/PolyXML/commit/78fe9767aef864e39d34b7003aef0be8dcc97fae)), closes [#41](https://github.com/nth-bailey/PolyXML/issues/41)

## [0.14.3](https://github.com/nth-bailey/PolyXML/compare/v0.14.2...v0.14.3) (2026-09-20)


### Bug Fixes

* **rust-codegen:** close attribute loop delimiter and match utf-8 attribute keys ([d0e9a87](https://github.com/nth-bailey/PolyXML/commit/d0e9a87c464f992c31a7a2da42456594d773ddbe))
* **rust-codegen:** use attr.key.local_name().as_ref() directly for str matching ([3409c3f](https://github.com/nth-bailey/PolyXML/commit/3409c3f681ae2ff20c3e3e23c7897da1bbea6d24))

## [0.14.2](https://github.com/nth-bailey/PolyXML/compare/v0.14.1...v0.14.2) (2026-09-20)


### Bug Fixes

* **codegen:** topologically sort typescript schemas by field dependencies and fix multiline doc comments ([902a31e](https://github.com/nth-bailey/PolyXML/commit/902a31e98c7de92d95da83bc94a59bbd3914cafc))
* **csharp:** add new keyword to Validate method on derived records to prevent CS0108 ([8533375](https://github.com/nth-bailey/PolyXML/commit/85333754f1efd617102c5f6052e22d6ab6cac1a4))
* **rust:** correct quick-xml 0.42 string matching, FromStr result, and enum default derive ([ccbb0fa](https://github.com/nth-bailey/PolyXML/commit/ccbb0fac9f1e4fe20f3c17fbdea2ba55f7c95251))

## [0.14.1](https://github.com/nth-bailey/PolyXML/compare/v0.14.0...v0.14.1) (2026-09-20)


### Bug Fixes

* **ci:** eliminate concurrent dotnet first-time initialization race in tests ([621668e](https://github.com/nth-bailey/PolyXML/commit/621668e54e94ba3558c542959af6f2efbc299872))

# [0.14.0](https://github.com/nth-bailey/PolyXML/compare/v0.13.0...v0.14.0) (2026-09-20)


### Features

* **transcoder:** dual-format XML ↔ JSON engine, annotations, codecs, and transcoder ([#39](https://github.com/nth-bailey/PolyXML/issues/39)) ([97f9fd0](https://github.com/nth-bailey/PolyXML/commit/97f9fd01fc6684ea889cb4f4d9cdf72d3a658422)), closes [#35](https://github.com/nth-bailey/PolyXML/issues/35) [#35](https://github.com/nth-bailey/PolyXML/issues/35) [#35](https://github.com/nth-bailey/PolyXML/issues/35) [#36](https://github.com/nth-bailey/PolyXML/issues/36) [#35](https://github.com/nth-bailey/PolyXML/issues/35) [#36](https://github.com/nth-bailey/PolyXML/issues/36) [#37](https://github.com/nth-bailey/PolyXML/issues/37) [#37](https://github.com/nth-bailey/PolyXML/issues/37) [#37](https://github.com/nth-bailey/PolyXML/issues/37) [#34](https://github.com/nth-bailey/PolyXML/issues/34) [#35](https://github.com/nth-bailey/PolyXML/issues/35) [#36](https://github.com/nth-bailey/PolyXML/issues/36) [#37](https://github.com/nth-bailey/PolyXML/issues/37)

# [0.13.0](https://github.com/nth-bailey/PolyXML/compare/v0.12.0...v0.13.0) (2026-09-20)


### Features

* **json:** add native JSON serialization and deserialization across PolyXML ([#38](https://github.com/nth-bailey/PolyXML/issues/38)) ([86a855c](https://github.com/nth-bailey/PolyXML/commit/86a855c59f47c0fc61268c3c9e52147463d406fa))

# [0.12.0](https://github.com/nth-bailey/PolyXML/compare/v0.11.1...v0.12.0) (2026-09-20)


### Features

* Next-Gen Polyglot XML Schema Compiler & Toolchain (Epic [#22](https://github.com/nth-bailey/PolyXML/issues/22)) ([#33](https://github.com/nth-bailey/PolyXML/issues/33)) ([ddfc204](https://github.com/nth-bailey/PolyXML/commit/ddfc204ea0d4feb197ed9e380cd193f5f6206455)), closes [#23](https://github.com/nth-bailey/PolyXML/issues/23) [#31](https://github.com/nth-bailey/PolyXML/issues/31) [#24](https://github.com/nth-bailey/PolyXML/issues/24) [#25](https://github.com/nth-bailey/PolyXML/issues/25) [#32](https://github.com/nth-bailey/PolyXML/issues/32) [#28](https://github.com/nth-bailey/PolyXML/issues/28) [#27](https://github.com/nth-bailey/PolyXML/issues/27) [#26](https://github.com/nth-bailey/PolyXML/issues/26) [#29](https://github.com/nth-bailey/PolyXML/issues/29) [#30](https://github.com/nth-bailey/PolyXML/issues/30)

## [0.11.1](https://github.com/nth-bailey/PolyXML/compare/v0.11.0...v0.11.1) (2026-09-17)


### Bug Fixes

* **core:** track unknown element subtree depth and strip duplicate prefixes in qualified serialization ([ee36649](https://github.com/nth-bailey/PolyXML/commit/ee366494739b14f00ad13a43f1d3a04864b3db6e))

# [0.11.0](https://github.com/nth-bailey/PolyXML/compare/v0.10.0...v0.11.0) (2026-09-17)


### Features

* multi-language feature parity for W3C XML namespaces across C, C++, Go, Node, and Java ([8965862](https://github.com/nth-bailey/PolyXML/commit/896586258ca535c36c1750d1ddbd8fa755369d8e))

# [0.10.0](https://github.com/nth-bailey/PolyXML/compare/v0.9.0...v0.10.0) (2026-09-17)


### Features

* **core,python:** implement toggleable W3C XML namespace support and prefix mapping ([fba695e](https://github.com/nth-bailey/PolyXML/commit/fba695e0a3697d6978c4b52738158ebff453407e))

# [0.9.0](https://github.com/nth-bailey/PolyXML/compare/v0.8.1...v0.9.0) (2026-09-13)


### Features

* add context7 configuration file ([5e01fd6](https://github.com/nth-bailey/PolyXML/commit/5e01fd689c7bad393251e6d523586a767782519e))

## [0.8.1](https://github.com/nth-bailey/PolyXML/compare/v0.8.0...v0.8.1) (2026-09-13)


### Bug Fixes

* **ci:** deduplicate release dispatch and handle concurrent maven central deployments ([7f92648](https://github.com/nth-bailey/PolyXML/commit/7f9264899e2800aa5f5b85904b892fa6c9192b8b))

# [0.8.0](https://github.com/nth-bailey/PolyXML/compare/v0.7.0...v0.8.0) (2026-09-13)


### Features

* **scripts:** add dual-language quality gate script ([07e6085](https://github.com/nth-bailey/PolyXML/commit/07e6085b8e1fc9591005fbb832cf189c71d45af3))

# [0.7.0](https://github.com/nth-bailey/PolyXML/compare/v0.6.0...v0.7.0) (2026-09-12)


### Features

* **core:** support mixed-content wildcard fields in deserializer and serializer ([529457f](https://github.com/nth-bailey/PolyXML/commit/529457f2793269d51cd7acd68620b296fb039f5e))

# [0.6.0](https://github.com/nth-bailey/PolyXML/compare/v0.5.1...v0.6.0) (2026-09-12)


### Features

* **python:** universal XML datatype support and class-tagging for binary MessagePack serialization ([b50e62d](https://github.com/nth-bailey/PolyXML/commit/b50e62d0fabe13bb6761c2666d220251c030eec5))

## [0.5.1](https://github.com/nth-bailey/PolyXML/compare/v0.5.0...v0.5.1) (2026-09-12)


### Bug Fixes

* **ci:** fix Homebrew release workflow tag resolution ([9e35fb9](https://github.com/nth-bailey/PolyXML/commit/9e35fb9ed5ea72e613bc6349a30a0fbd6d09388c))
* **ci:** upgrade bump-homebrew-formula-action v3 → v4 ([7f863c4](https://github.com/nth-bailey/PolyXML/commit/7f863c45273fa45e99bd53f34ac5e49a7f4c9d83))

# [0.5.0](https://github.com/nth-bailey/PolyXML/compare/v0.4.0...v0.5.0) (2026-09-12)


### Features

* **binary:** add zero-GIL binary serialization (dumps_binary, loads_binary) with 100% test coverage ([f3c115a](https://github.com/nth-bailey/PolyXML/commit/f3c115aff96a43821562b5b003c981e364df35e8))

# [0.4.0](https://github.com/nth-bailey/PolyXML/compare/v0.3.0...v0.4.0) (2026-09-10)


### Features

* upgrade quick-xml to 0.42, criterion to 0.8, and pyo3 to 0.29 ([440bb14](https://github.com/nth-bailey/PolyXML/commit/440bb1489286a0ddd97f04ca8dce3067bac271d8))

# [0.3.0](https://github.com/nth-bailey/PolyXML/compare/v0.2.1...v0.3.0) (2026-09-10)


### Features

* upgrade quick-xml to 0.37, pyo3 to 0.23, criterion to 0.7 and update dependabot rules ([5e25ff5](https://github.com/nth-bailey/PolyXML/commit/5e25ff522542fb258445a03080b9882f3866bfc5))

## [0.2.1](https://github.com/nth-bailey/PolyXML/compare/v0.2.0...v0.2.1) (2026-09-10)


### Bug Fixes

* **python:** make date/time resolution standalone and ignore breaking cargo major updates in dependabot ([6d04cc8](https://github.com/nth-bailey/PolyXML/commit/6d04cc8d302f2e876152611ffd48014d9ff089d5))

# [0.2.0](https://github.com/nth-bailey/PolyXML/compare/v0.1.0...v0.2.0) (2026-09-10)


### Bug Fixes

* **ci:** make PyPI publish idempotent with skip-existing ([e5c2b32](https://github.com/nth-bailey/PolyXML/commit/e5c2b32459ce54f6f0792d69b85a1be0105f4e1e))


### Features

* **conan:** add Conan 2.0 recipe, ConanCenter package files, and test package ([325cf72](https://github.com/nth-bailey/PolyXML/commit/325cf72b155df356c51800d57139636f8ec4bc2e))
* **packaging:** add AUR PKGBUILD, .SRCINFO, and update packaging recipes ([11a8446](https://github.com/nth-bailey/PolyXML/commit/11a84460595be39f1a23e05fdac17cdf44f97d1a))
* **packaging:** add conda-forge, vcpkg, and Homebrew distribution packages ([c71e3fb](https://github.com/nth-bailey/PolyXML/commit/c71e3fbb1bd9e34ac87e8f2d845e691f1e0b2fe7))
* **perf:** implement streaming iterparse and bridge optimizations ([df5fff9](https://github.com/nth-bailey/PolyXML/commit/df5fff936a6f50eb0c5c90e476585cdf8abab5b8))
* **python:** add rich type support, dataclass kw_only, and stream sources parity with pyxsdata-core ([0700104](https://github.com/nth-bailey/PolyXML/commit/0700104d815c621b0f310f313352a4f37c4b80f3))
