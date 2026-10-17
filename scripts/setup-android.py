#!/usr/bin/env python3
"""Install the project's Linux Android toolchain in the user's data directory."""
import hashlib
import os
from pathlib import Path
import shutil
import subprocess
import tarfile
import urllib.request
import zipfile

ROOT = Path.home() / '.local/share/holocron'
TOOLS = ROOT / 'toolchains'
SDK = ROOT / 'android-sdk'
JDK = TOOLS / 'jdk-21.0.12.1+1'
DOWNLOADS = ROOT / 'downloads'


def archive(name, url, expected, algorithm='sha256'):
    DOWNLOADS.mkdir(parents=True, exist_ok=True)
    target = DOWNLOADS / name
    if not target.exists():
        print(f'Downloading {name}', flush=True)
        partial = target.with_suffix(target.suffix + '.partial')
        urllib.request.urlretrieve(url, partial)
        partial.rename(target)
    with target.open('rb') as stream:
        actual = hashlib.file_digest(stream, algorithm).hexdigest()
    if actual != expected:
        raise RuntimeError(f'Checksum mismatch: {target}; remove this download and retry')
    return target


def unzip(source, destination):
    with zipfile.ZipFile(source) as package:
        package.extractall(destination)
        for item in package.infolist():
            mode = item.external_attr >> 16
            if mode:
                (destination / item.filename).chmod(mode & 0o777)


def main():
    if os.uname().sysname != 'Linux' or os.uname().machine != 'x86_64':
        raise SystemExit('This bootstrap supports Linux x86_64 only.')
    TOOLS.mkdir(parents=True, exist_ok=True)
    SDK.mkdir(parents=True, exist_ok=True)
    if not (JDK / 'bin/javac').exists():
        source = archive('temurin-21.0.12.1+1.tar.gz',
            'https://github.com/adoptium/temurin21-binaries/releases/download/jdk-21.0.12.1%2B1/OpenJDK21U-jdk_x64_linux_hotspot_21.0.12.1_1.tar.gz',
            'ce79869e1307ed8ee1e2baa86a412b1eb5b75d10a01006d788a6f968bcfaee94')
        with tarfile.open(source) as package:
            package.extractall(TOOLS, filter='data')
    manager = SDK / 'cmdline-tools/19.0/bin/sdkmanager'
    if not manager.exists():
        source = archive('commandlinetools-linux-13114758.zip',
            'https://dl.google.com/android/repository/commandlinetools-linux-13114758_latest.zip',
            '7ec965280a073311c339e571cd5de778b9975026cfcbe79f2b1cdcb1e15317ee')
        staging = TOOLS / 'android-cli-staging'
        staging.mkdir(exist_ok=True)
        unzip(source, staging)
        manager.parent.parent.parent.mkdir(parents=True, exist_ok=True)
        shutil.move(str(staging / 'cmdline-tools'), manager.parent.parent)
        staging.rmdir()
    gradle = TOOLS / 'gradle-8.13/bin/gradle'
    if not gradle.exists():
        source = archive('gradle-8.13-bin.zip',
            'https://services.gradle.org/distributions/gradle-8.13-bin.zip',
            '20f1b1176237254a6fc204d8434196fa11a4cfb387567519c61556e8710aed78')
        unzip(source, TOOLS)
    env = dict(os.environ, JAVA_HOME=str(JDK), ANDROID_HOME=str(SDK))
    # Installing the requested Android development tools includes their SDK licenses.
    subprocess.run([str(manager), f'--sdk_root={SDK}', '--licenses'],
                   input='y\n' * 100, text=True, env=env, check=True)
    subprocess.run([str(manager), f'--sdk_root={SDK}',
        'platform-tools', 'emulator', 'platforms;android-36', 'build-tools;35.0.0',
        'ndk;28.2.13676358', 'system-images;android-36;google_apis;x86_64'],
        env=env, check=True)
    print('Android tools installed. Source scripts/android-env.sh before building.')


if __name__ == '__main__':
    main()
