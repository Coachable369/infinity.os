# InfinitySDK

Native application development for InfinityOS — SDK version 1.0.0.

Open `/InfintySDK` in File Navigator on either ARM64 or x86_64. This spelling
is the public installation path. The SDK is available to every signed-in user,
read-only and versioned with the operating system. Copy individual examples to
your Documents folder before editing or running them. Updating the OS updates
the SDK; it never overwrites your copies.

## Start here

1. Read `APIKit/GettingStarted.md`.
2. Copy `Examples/Python/counter.py` into your Documents folder.
3. Open the copy in Text Editor and select Run. Python's `infinity` module is
   already installed; no pip package or internet connection is required.
4. Read `AIKit/Integration.md` to expose the same application actions to local AI.

## Contents

| Kit | Contents |
| --- | --- |
| UIKit | The OS's authoritative Eclipse Precision specification, component catalog, style payload, atlas, reference artwork and public UI headers |
| APIKit | Native C headers, Python module, API reference and development guide |
| AIKit | Local AI interaction contract, typed action header, capabilities and security guidance |
| AudioKit | PCM/WAV quick start, tested tone-generation example, format reference and explicit native binding limitations |
| CameraKit | Authorized camera quick start, paged capture API reference and error/lifecycle guidance |
| LogKit | Bounded JSON diagnostics in native Python and C, with console delivery and error handling |
| NetworkKit | Authorized native IPv4 TCP/DNS/TLS client APIs and examples in both languages |
| Examples | C and Python applications using the real native APIs |
| manifest.json | Package version, source mappings, byte counts and SHA-256 checksums |

Each kit has a starting guide and reference: `APIKit/GettingStarted.md` and
`APIKit/Reference.md`; `UIKit/QuickStart.md` and its shared specifications;
`AIKit/QuickStart.md` and `AIKit/Integration.md`; `AudioKit/README.md`; and
`CameraKit/README.md`. Runnable examples are under `Examples/C` and
`Examples/Python`. Guides distinguish public app APIs from internal OS services
and list expected outcomes, permission requirements and troubleshooting steps.

The archive distribution has the same relative paths and bytes as the installed
SDK. `./build-kit sdk-package` creates `builds/InfintySDK.tar.gz` and its checksum.
The source catalog is `sdk/distribution/package.json`; copied documentation is
not a second implementation of UIKit. Native applications use the installed OS
renderer, fonts and current theme rather than drawing their own window chrome.

## Compatibility and limits

Version 1 targets the existing native InfinityOS application ABI on both CPU
architectures. Applications must not assume a Linux, macOS, Win32 or POSIX host.
See APIKit's support matrix before selecting libraries. The SDK does not bundle
a second compiler, Python interpreter, LLM, or model weights: those are OS
services. SDK assets are immutable package resources and consume no personal
document slots. Large binary resources are intended for image viewers/export,
not Text Editor. Copying individual text examples is supported; recursive SDK
folder copying is not supported by the bounded object store.

## One SDK for developers and AI

Python and C applications use native OS services, not an emulation layer or host
GUI framework. `AIKit/NativeAppGuide.txt` is also embedded in the OS application
generation prompt. It points to these same references, documents actual supported
APIs and prohibits invented APIs or self-granted permissions. Code generation is
not proof of execution: compile/validate, run and check observable results.

## Release and licensing

The manifest records exact content, not a promise of ABI compatibility with
future major releases. Headers and examples retain repository licensing;
upstream components retain their notices. This distribution does not grant a
new license to redistribute proprietary artwork or third-party model weights.
See `LICENSES.md` for the included material and source locations.
