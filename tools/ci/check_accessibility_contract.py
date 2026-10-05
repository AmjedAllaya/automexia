#!/usr/bin/env python3
"""Pin the reviewed native boundary without treating it as registry source."""
from __future__ import annotations

import hashlib
import os
from pathlib import Path
import stat
import sys
import tomllib

ROOT = Path(__file__).resolve().parents[2]
VENDOR = "third-party/accesskit-windows"
MAX_FILE_BYTES = 128 * 1024
MAX_TOTAL_BYTES = 512 * 1024
# Canonical UTF-8/LF text digests; updating these requires source review.
VENDOR_HASHES = {
    'Cargo.toml': '3b3af85187449beba705b21dcd5882ac56813a86542717b030e63a8786094507',
    'LICENSE-APACHE': '62c7a1e35f56406896d7aa7ca52d0cc0d272ac022b5d2796e7d6905db8a3636a',
    'LICENSE-MIT': '23f18e03dc49df91622fe2a76176497404e46ced8a715d9d2b67a7446571cca3',
    'LICENSE.chromium': '845022e0c1db1abb41a6ba4cd3c4b674ec290f3359d9d3c78ae558d4c0ed9308',
    'README.md': '622db8e92d711b64279a91e25ae9345959c82bb4d47157dbf5c3a6a53f80f6d6',
    'src/adapter.rs': '01b2600adff2fce3a7aec0249a8e5aef73ec14f42743c0c1d57609df374adb55',
    'src/context.rs': '73f61607ff93ab7876f60a52ed01f9ee4a1675ed080eb2f8ec98971854833514',
    'src/filters.rs': 'fa1c7caaf2b5ff0d6037d773259ace479e58cd1aead31f9b3dda220d467bc363',
    'src/lib.rs': 'd4392137a99bfa04905a491b5f73655c7e647928b269fc9df2e5c58d59990ec7',
    'src/node.rs': '967285756a334e85f01b24ffa4b9b52f9bdb327b21b507a15848378a0059604a',
    'src/subclass.rs': '4c7eedb5965fbf6a4a62473821334c015e6468aaea4c2f775f3b9072870777c1',
    'src/tests/mod.rs': '051e571de660dcd5abe6f43c9cbbf8f835c76528ab47e1620837fec3d74cb480',
    'src/tests/simple.rs': 'a044abb981df3005c82ad898d96b4b19ecf948714844ae52cf1570306c0f2384',
    'src/tests/subclassed.rs': 'c1694fd0bcf65a4789741ca9fda27c2a193a28dc18b2b852df59500d6f396188',
    'src/text.rs': 'e5daefece532bdb6b330b03338cd1a3531541efa905912d701eaea4d8a3f13bc',
    'src/util.rs': '0c62979e8189dcb2a75935f998d59ad32ba6f33cd2277266b595725d5c0866ae',
    'src/window_handle.rs': 'dec1d78c81784a18c50f066e89d57d0fe02a7c9199a11e47a0833f8b9113d8c2',
    'UPSTREAM.md': 'd0ce62855b26e01537a1ce35dc71284cd1734328ada708a756bc05deaffa8d9a',
}


ADDITIONAL_ADAPTERS = {
    'third-party/accesskit-unix': ('accesskit_unix', '0.24.0', {
        'Cargo.toml': 'b0d829eb7a30648e625107be4d6dd7aefc40bd814f2d3b6e633b13578261020b',
        'LICENSE-APACHE': '62c7a1e35f56406896d7aa7ca52d0cc0d272ac022b5d2796e7d6905db8a3636a',
        'LICENSE-MIT': '23f18e03dc49df91622fe2a76176497404e46ced8a715d9d2b67a7446571cca3',
        'README.md': 'e13f4f08fb4f1c593f1303b6e669d476ca810822448272078858b6690d48b234',
        'src/adapter.rs': 'b893f84db714f1db4298d173e5e9c6fd6bfbcdaae38c0677883db36c93601c97',
        'src/atspi/bus.rs': 'a6ab7647494963b4037e51976581a957b9e14f17ce5d435a9599cec8811ea96d',
        'src/atspi/interfaces/accessible.rs': 'bdcd776f031335aeb17372cc19a9a23ff2752ca145d09113ba6b42677c751bb0',  # gitleaks:allow - verified source SHA-256, enforced below
        'src/atspi/interfaces/action.rs': '7ffa334b1cace21aa567a074d0c1ba59dd2229449af02ffc329bfd4671940903',
        'src/atspi/interfaces/application.rs': '530316441ee42b415605f6f50526ac776d8ab038aff24dfa02d4352dedcf2db0',
        'src/atspi/interfaces/cache.rs': '0b06555ad8f4fc33b7ce19565c533f63f5dd38cddfd5c3673e3ef208f42aed0e',
        'src/atspi/interfaces/component.rs': 'c69fa80aba188c445efb3f8731c5dbbd58c670c2252fc364d549a6807feb0911',
        'src/atspi/interfaces/document.rs': '61cde20381ea0966adb5ed67ad262a7d8231c10c44cbeecda639316154618f1a',
        'src/atspi/interfaces/editable_text.rs': '50868d2d56bb5415105a8e61f527510984a9fcdd94c4627bf7232ce7bfc13eb1',
        'src/atspi/interfaces/hyperlink.rs': '9a53b1d164208963ed047326f12ab380700807a3338b8c76f4edd8a492cd35d8',
        'src/atspi/interfaces/image.rs': '29cbbe368afbbbb8bfe9a8ede4dc4a2154c329e136e6aa5f115a37aa5b403b0e',
        'src/atspi/interfaces/mod.rs': 'a3f7875a51c3f69de9a6cbbf03044fa588270d6095c00446525308d84a5172fb',
        'src/atspi/interfaces/selection.rs': 'a15f322ad9edd16f5bc13d9afd7c3e5536abde08cd777c550c5c00c399987f3c',
        'src/atspi/interfaces/text.rs': 'd56bf8e522f98e07d7c8d9f91b528b3388b13137d919c4092f614ec71676fa84',
        'src/atspi/interfaces/value.rs': '829dfa73ab581da1be9210c1a6a0361bbb8563fb8d49c7057557cc1ddefafbab',
        'src/atspi/mod.rs': '2ea8e56ae5d3fde6a380582890597d49a0390a86dfddcb275df5281ad46cd90e',
        'src/atspi/object_address.rs': '3bd0dde2616fcc1dd48e984c62dcaee871a646f890e51f28aeff7017bbd05d55',
        'src/atspi/object_id.rs': '8799ebd29d8b269fba072eb11b08c4b311afd035852ab4530450e3a03e9622e9',
        'src/context.rs': '59aafdd71828154ce91f044e068474ab203eb6bf9a820f4cd70f60721c86b1b6',
        'src/executor.rs': '91a22d12d280357e8d2715d4ec33068d49e1a07ba285912f0ef228dd959a5310',
        'src/lib.rs': '15b7b26dcc67269bf1b2e22326aeefa65e1c3ca0364fbf433293ff079d9da779',
        'src/queue.rs': '5e8c3bda29ced1f4643fdd4220d37030cb71151fcd29ae04307bc14086adfd5d',
        'src/util.rs': 'dd6eb128d8a0192d61a76722341b3cb4c02ae7f04580d20be1270abdfde8181c',
        'UPSTREAM.md': '418bdb3afc687382713741b9506bc8004f182f2e1e54c692f598a90b7dde9017',
    }),
    'third-party/accesskit-macos': ('accesskit_macos', '0.27.1', {
        'Cargo.toml': '2e860984649abc7ba7c054f492811f0c7e3f6dfda0dc83557e9c1f550d970acc',
        'examples/native_accessibility_smoke.rs': '02601744aedd899114876b3e5644add4d510c862dfda966c09c8c65da96160f8',
        'LICENSE-APACHE': '62c7a1e35f56406896d7aa7ca52d0cc0d272ac022b5d2796e7d6905db8a3636a',
        'LICENSE-MIT': '23f18e03dc49df91622fe2a76176497404e46ced8a715d9d2b67a7446571cca3',
        'LICENSE.chromium': '845022e0c1db1abb41a6ba4cd3c4b674ec290f3359d9d3c78ae558d4c0ed9308',
        'README.md': 'ac9a875391361e0cc0a0cae754cf5cc451d24f1c7f7b70c8f16c7601ec53bfd4',
        'src/adapter.rs': 'a53b178b302c9630f74f5eed908afced165269dfe9108bed03520a00140ab32f',
        'src/boundary.rs': 'bec9d54998c56c38801eed78fc72a665cbf24f93b184827b22844307c333619d',
        'src/context.rs': '23287f58bbd05b6e6ade7b333c384aa2ecfe19c8c7ea2b8a3ddb778c927fcedf',
        'src/event.rs': 'df621fd1c5b1404812d84e1eb1dc93ba055f9fc386e051c627438f0357d26536',
        'src/filters.rs': 'cfe74d16da43ccfda209f11107faf78442c6830fa126cf2d48ffdcdf14c65288',
        'src/lib.rs': 'faaf2aa2ce137385b7999945bc725fa8287eee7c31ee0501a01f20df7770a18a',
        'src/node.rs': '292732c9b4d79ecee2821358bc76c9bdf3aac1b2d3a34bff23a03514879c5fb2',
        'src/patch.rs': '66cd13504699cb38789cdb07f7cb2c7878c0ef0f867a4f6040b3d014ef63d7d6',
        'src/subclass.rs': '1ad7a5028b553e894017954a83c0f5217f5c1ed7152a9274dde76f994cc4d12a',
        'src/util.rs': 'd44bf7569b47e7da37495ec134a2955abff80a7011e7811cd872db9b42e87e8f',
        'UPSTREAM.md': '9868a21cfa336d7c9bdd91f70fc5e31b939228ecccb73f748e6f7f78a53bf87f',
    }),
}


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def bundle(root: Path, vendor: str = VENDOR) -> dict[str, str]:
    base = root / vendor
    require(base.is_dir() and not base.is_symlink() and not getattr(base, "is_junction", lambda: False)(), "native adapter directory missing or linked")
    result: dict[str, str] = {}
    total = 0
    visited = 0
    for current, directories, files in os.walk(base, followlinks=False):
        visited += len(directories) + len(files)
        require(visited <= (32 if vendor == VENDOR else 40), "native adapter inventory exceeds bound")
        for name in directories:
            directory = Path(current) / name
            require(not directory.is_symlink() and not getattr(directory, "is_junction", lambda: False)(), "native adapter linked directory")
        for name in files:
            path = Path(current) / name
            info = path.lstat()
            require(stat.S_ISREG(info.st_mode) and not path.is_symlink() and info.st_nlink == 1,
                    "native adapter requires regular unlinked files")
            require(info.st_size <= MAX_FILE_BYTES, "native adapter file exceeds bound")
            total += info.st_size
            require(total <= MAX_TOTAL_BYTES, "native adapter source exceeds bound")
            raw = path.read_bytes()
            require(len(raw) == info.st_size, "native adapter file changed while reading")
            result[path.relative_to(base).as_posix()] = raw.decode('utf-8').replace('\r\n', '\n')
    return result


def validate(files: dict[str, str], manifest: str, lock: str, policy: str, model: str, app: str,
             additional: dict[str, dict[str, str]]) -> None:
    require(set(files) == set(VENDOR_HASHES), "native adapter reviewed file inventory changed")
    for name, expected in VENDOR_HASHES.items():
        require(hashlib.sha256(files[name].encode()).hexdigest() == expected,
                f"native adapter source review required: {name}")
    cargo = tomllib.loads(manifest)
    require(cargo['patch']['crates-io']['accesskit_windows'] == {'path': VENDOR},
            "native adapter reviewed override missing")
    require(VENDOR in cargo['workspace']['members'], "native adapter tests must run in workspace")
    require(cargo['workspace']['dependencies']['accesskit'] == '=0.25.1', "semantic schema pin changed")
    packages = [p for p in tomllib.loads(lock)['package'] if p['name'] == 'accesskit_windows']
    require(len(packages) == 1 and packages[0]['version'] == '0.35.1' and 'source' not in packages[0],
            "native adapter registry substitution or version drift")
    require(tomllib.loads(policy)['policy']['accesskit_windows']['audit-as-crates-io'] is False,
            "modified source cannot masquerade as registry certification")
    require(set(additional) == set(ADDITIONAL_ADAPTERS), "native adapter inventory missing")
    for vendor, (package, version, hashes) in ADDITIONAL_ADAPTERS.items():
        source = additional[vendor]
        require(set(source) == set(hashes), "native adapter reviewed file inventory changed")
        for name, expected in hashes.items():
            require(hashlib.sha256(source[name].encode()).hexdigest() == expected,
                    f"native adapter source review required: {vendor}/{name}")
        require(cargo['patch']['crates-io'][package] == {'path': vendor}, "native adapter reviewed override missing")
        require(vendor in cargo['workspace']['members'], "native adapter tests must run in workspace")
        entries = [p for p in tomllib.loads(lock)['package'] if p['name'] == package]
        require(len(entries) == 1 and entries[0]['version'] == version and 'source' not in entries[0],
                "native adapter registry substitution or version drift")
        require(tomllib.loads(policy)['policy'][package]['audit-as-crates-io'] is False,
                "modified source cannot masquerade as registry certification")
    model = tomllib.loads(model)
    require(model['dependencies']['accesskit'] == {'workspace': True}, "shared semantic schema owner changed")
    for name in ('loom', 'proptest'):
        require(name in model['dev-dependencies'] and name not in model['dependencies'],
                "model assurance tools must be development-only")
    require(not any(name.startswith('accesskit_') for name in model['dependencies']),
            "native adapters must stay outside the pure model")
    app = tomllib.loads(app)
    for target, name, version in (
        ('cfg(windows)', 'accesskit_windows', '=0.35.1'),
        ('cfg(target_os = "macos")', 'accesskit_macos', '=0.27.1'),
        ('cfg(all(unix, not(target_os = "macos")))', 'accesskit_unix', '=0.24.0'),
    ):
        require(app['target'][target]['dependencies'][name] == version and name not in app['dependencies'],
                "native adapter version or platform boundary changed")


def inputs(root: Path = ROOT) -> tuple:
    names = ('Cargo.toml', 'Cargo.lock', 'supply-chain/config.toml',
             'automexia-ui-model/Cargo.toml', 'apps/automexia-terminal/Cargo.toml')
    return (bundle(root), *( (root / name).read_text(encoding='utf-8') for name in names ),
            {vendor: bundle(root, vendor) for vendor in ADDITIONAL_ADAPTERS})


def main() -> int:
    try:
        validate(*inputs())
    except (OSError, ValueError, KeyError, TypeError) as error:
        print(f"accessibility boundary validation failed: {error}", file=sys.stderr)
        return 1
    print("PASS: native adapter provenance, source inventory, platform and pure-model boundaries")
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
