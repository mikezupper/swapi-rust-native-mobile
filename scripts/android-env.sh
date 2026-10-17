#!/usr/bin/env bash
# Source this file. Machine-specific overrides may be supplied by the caller.
export HOLOCRON_TOOLS="${HOLOCRON_TOOLS:-$HOME/.local/share/holocron/toolchains}"
export JAVA_HOME="${JAVA_HOME:-$HOLOCRON_TOOLS/jdk-21.0.12.1+1}"
export ANDROID_HOME="${ANDROID_HOME:-$HOME/.local/share/holocron/android-sdk}"
# Hosted runners can inherit this deprecated alias for a different SDK.
# Keep tools that still inspect it consistent with the selected ANDROID_HOME.
export ANDROID_SDK_ROOT="$ANDROID_HOME"
export ANDROID_NDK_HOME="$ANDROID_HOME/ndk/28.2.13676358"
export PATH="$JAVA_HOME/bin:$ANDROID_HOME/cmdline-tools/19.0/bin:$ANDROID_HOME/platform-tools:$ANDROID_HOME/emulator:$PATH"
