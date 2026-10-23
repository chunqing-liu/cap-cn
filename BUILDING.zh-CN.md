# Windows x64 构建

本文描述当前 `0.6.1-cn.2` 构建方式。上游其他平台和云服务的开发说明仍在 `README.upstream.md`。完整原生构建资源消耗较大，工具测试通过不代表原生构建成功。

复现双屏编辑增强预览版时，将第 2 节的检出标签换成 `v0.6.1-cn.2-dual-preview.1`，并在第 4 节命令末尾使用 `-- -Ccodegen-units=16 -Copt-level=1`。程序内部沿用基础版本 `0.6.1-cn.2`；预览发行身份、验证范围和已知限制见[预览版说明](tools/zh-cn/releases/dual-preview.md)。第 6 节的稳定版安装脚本不会自动生成预览版发布物，不能将其默认输出冒充本次预览包。

## 1. 工具链

- Windows x64，PowerShell 7、Git、Node.js 22 或更高版本。
- Rust MSVC **1.95.0**；根目录旧的 `rust-toolchain.toml` 不用于此次 GPUI 构建，下面显式指定工具链。
- Visual Studio 2022 C++ Build Tools，MSVC v143、Windows SDK、LLVM / libclang、CMake。当前构建使用 MSVC 14.44 与 Windows SDK 10.0.26100。
- 如需安装整个上游 JavaScript 工作区，再按上游说明安装 Bun；原生依赖准备入口是 `node scripts/setup.js`。
- 打包需要 Inno Setup **6.4.3**、微软允许再分发的 VC143 x64 CRT，以及匹配版本的 `cap-muxer.exe`。

在 **Developer PowerShell for VS 2022（x64）** 中运行后续构建步骤。根据自己的安装位置设置 `LIBCLANG_PATH` 和 `CMAKE`，不要复制其他电脑的绝对路径。

## 2. 获取源码与 GPUI

```powershell
git clone https://github.com/chunqing-liu/cap-cn.git
cd cap-cn
git checkout v0.6.1-cn.2
pwsh -NoProfile -File .\tools\zh-cn\prepare-gpui.ps1
```

脚本在仓库旁创建 `Cap-cn-gpui`，获取指定基础提交并应用完整补丁，与 `apps/desktop-gpui/Cargo.toml` 的相对依赖路径一致。如果该目录已存在，脚本会停止，不覆盖现有改动。

不要使用上游的 `build-gpui-binary.sh` 构建中文版本：它会同步官方版本号，并准备另一套 GPUI 目录。

## 3. 准备媒体依赖

```powershell
$env:RUST_TARGET_TRIPLE = "x86_64-pc-windows-msvc"
node .\scripts\setup.js
```

官方准备脚本会下载媒体依赖并生成未纳入 Git 的 `.cargo/config.toml`。下载或本机工具检测失败时，先按报错修复，不应继续打包缺少依赖的版本。

当前发布构建使用：

- Gyan FFmpeg **7.1 full shared**：`target/ffmpeg`；头文件和 MSVC 导入库位于 `target/native-deps/include` 与 `target/native-deps/lib`。
- DirectX Shader Compiler **1.9.2607.13**：`target/native-deps/dxc`。
- ONNX Runtime **1.24.2**：`target/native-deps/onnxruntime/lib`。

`scripts/setup.js` 的依赖版本可能随上游改变。若需要复现此次发布，应对应 `tools/zh-cn/third-party/` 的版本及原始发行说明，而非混用最新 DLL。可能还需根据本机 Visual Studio 位置补充 `LIBCLANG_PATH` / `CMAKE`。脚本需要网络，不保证在所有网络环境自动完成。

## 4. 编译主程序

```powershell
rustup toolchain install 1.95.0 --profile minimal
rustup target add x86_64-pc-windows-msvc --toolchain 1.95.0
cd apps\desktop-gpui
cargo +1.95.0-x86_64-pc-windows-msvc rustc --release --target x86_64-pc-windows-msvc --features local-chinese-trial --bin cap-gpui --jobs 6 -- -Ccodegen-units=16
cd ..\..
```

产物：`apps/desktop-gpui/target/x86_64-pc-windows-msvc/release/cap-gpui.exe`。

不要直接运行上游版本号同步脚本，也不要关闭 LTO 来复用已构建的 release 依赖；这些做法会导致版本号或链接输入不匹配。

## 5. 工具测试

```powershell
node --test .\tools\zh-cn\localize-ui.test.mjs
pwsh -NoProfile -File .\tools\zh-cn\test-maintenance.ps1
```

迁移测试只创建自己的临时测试目录，不读取个人录屏。正式发布前仍需检查原生界面、录制、编辑、导出、覆盖安装和数据恢复。

## 6. 整理程序与安装包

准备与上游 **0.6.1** 匹配的官方程序目录，用于取得未修改的 `cap-muxer.exe`；或者自行从本仓库相应 crate 构建配套程序。当前 `package-trial.ps1` 会检查指定官方目录的版本。

```powershell
pwsh -File .\tools\zh-cn\package-trial.ps1 -OfficialDirectory "<官方 Cap 0.6.1 程序目录>"
pwsh -File .\tools\zh-cn\build-installer.ps1 `
  -CompilerPath "<Inno Setup 6>\ISCC.exe" `
  -CrtDirectory "<VC Redist>\x64\Microsoft.VC143.CRT" `
  -GpuiDirectory "..\Cap-cn-gpui"
```

脚本会生成 setup、portable 和 maintenance 三种产物，只打包程序及默认配置，不读取现有 `profile`。已有输出不会被覆盖，可指定新的 `-OutputDirectory`。

安装程序为每用户安装，不接管官方版，不携带数字签名。打包不等于发布；上传前应检查实际文件、许可、源码对应关系及个人数据排除情况。
