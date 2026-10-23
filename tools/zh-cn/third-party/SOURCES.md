# 第三方组件、许可与源码

Cap 中文版是基于 Cap 的非官方修改构建。Cap 的原始版权、AGPLv3 以及仓库内的 MIT / 第三方例外保持不变；本项目没有取得 Cap 官方背书。

## 主程序与配套程序

- 修改后的完整 Cap 源码：[chunqing-liu/cap-cn](https://github.com/chunqing-liu/cap-cn)，稳定标签 `v0.6.1-cn.2`；双屏编辑增强预览标签 `v0.6.1-cn.2-dual-preview.1`。请按所用发行包选择对应源码。
- 原始版本：[CapSoftware/Cap cap-v0.6.1](https://github.com/CapSoftware/Cap/tree/cap-v0.6.1)，提交 `cb12972577993e6051228a703af17e59564b9289`。
- `Cap.exe` 遵循仓库根目录的许可；`cap-muxer.exe` 是相同官方版本的未修改配套程序，其源码在 `crates/cap-muxer` 与 `crates/cap-muxer-protocol`。
- GPUI 源码来自 [wingleeio/zed](https://github.com/wingleeio/zed/tree/5d1f83d9f27a19bec1fb241dc33b42238af9cf8d)，完整修改在 `source-tools/gpui-local.patch`；GPUI 使用 Apache-2.0，Zed 仓库其他部分按各自许可处理。`prepare-gpui.ps1` 可获取并应用该补丁。
- Rust / JavaScript 依赖的版本记录在随源码提供的锁文件中，各组件仍保留各自许可。

## FFmpeg

- 未修改的 Gyan FFmpeg **7.1 full shared** Windows x64 构建，GPLv3，不是 nonfree 构建。
- 原始二进制发行：[Gyan FFmpeg release builds](https://www.gyan.dev/ffmpeg/builds/#release-builds)。
- FFmpeg 对应源码：[FFmpeg commit b08d7969c5](https://github.com/FFmpeg/FFmpeg/tree/b08d7969c5)，[源码压缩包](https://github.com/FFmpeg/FFmpeg/archive/b08d7969c5.tar.gz)。
- `ffmpeg/README.txt` 保留原发行方的完整构建配置、外部库清单及具体版本，`ffmpeg/LICENSE-GPLv3.txt` 保留 GPLv3 正文。
- 外部库的源码与构建入口见 [media-autobuild_suite](https://github.com/m-ab-s/media-autobuild_suite)、[Gyan 构建说明](https://www.gyan.dev/ffmpeg/builds/) 和 [FFmpeg external libraries](https://ffmpeg.org/general.html#External-libraries)。获取源码时应对应 README 中列明的版本，而不是直接使用各库最新分支。

## DirectX Shader Compiler

- Microsoft.Direct3D.DXC **1.9.2607.13**，未修改的 `dxcompiler.dll` / `dxil.dll`。
- [官方发行包](https://www.nuget.org/packages/Microsoft.Direct3D.DXC/1.9.2607.13)；[源码仓库](https://github.com/microsoft/DirectXShaderCompiler)。
- 许可正文保存在 `dxc/LICENSE-LLVM.txt` 和 `dxc/LICENSE-MS.txt`。

## ONNX Runtime

- ONNX Runtime **1.24.2**，未修改的 Windows x64 动态库。
- [对应源码](https://github.com/microsoft/onnxruntime/tree/v1.24.2)；[官方发行](https://github.com/microsoft/onnxruntime/releases/tag/v1.24.2)。
- MIT 许可、第三方声明及原始源码提交记录保存在 `onnxruntime/`。

## Microsoft Visual C++ 运行库

- 安装包附带 app-local Microsoft VC143 x64 CRT，版本 **14.44.35211.0**，不替换系统文件。
- 这些运行库不是本项目的 AGPL 源码，适用微软自己的再分发条款。
- [再分发许可与文件说明](https://learn.microsoft.com/cpp/windows/redistributing-visual-cpp-files)。
- [官方支持的运行库下载与条款](https://learn.microsoft.com/cpp/windows/latest-supported-vc-redist)。

## 安装程序

- 使用 [Inno Setup 6.4.3](https://github.com/jrsoftware/issrc/tree/is-6_4_3) 生成。
- 简体中文语言文件保留原维护者 Zhenghan Yang 的署名与来源。
- 本项目没有发布 Inno Setup 编译器本体；安装程序及脚本分别受各自许可约束。
