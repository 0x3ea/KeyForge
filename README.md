# KeyForge

KeyForge 是一个确定性密码生成器。它根据以下输入生成站点密码：

- master password
- site
- username
- length
- symbols 符号集

相同输入会稳定生成相同密码，因此不需要保存每个网站的密码；但也意味着你必须牢记 master password，并保持 site / username / length / symbols 配置一致。

## 目录

- [功能](#功能)
- [安装](#安装)
  - [从源码安装](#从源码安装)
  - [从 GitHub Release 安装](#从-github-release-安装)
- [使用](#使用)
  - [生成并打印密码](#生成并打印密码)
  - [指定 username](#指定-username)
  - [指定密码长度](#指定密码长度)
  - [启用符号](#启用符号)
  - [复制到剪贴板](#复制到剪贴板)
  - [记住站点配置](#记住站点配置)
  - [生成 shell completion](#生成-shell-completion)
- [配置文件](#配置文件)
- [Site 归一化规则](#site-归一化规则)
- [派生算法与稳定性](#派生算法与稳定性)
- [未来计划](#未来计划)
- [开发](#开发)
- [使用风险和注意事项](#使用风险和注意事项)

## 功能

- Argon2id 密钥派生
- HMAC-SHA256 扩展输出
- 拒绝采样生成密码字符，避免取模偏差
- 覆盖保证：小写、大写、数字以及启用的符号至少出现一次
- 内置符号字符集：`!@#$%^&*-_=+`
- 符号集可按站自定义，也可继承默认配置
- master password 隐藏输入和二次确认
- 默认将 username 转为小写
- site 归一化为 hostname 小写形式
- 可将密码打印到终端或复制到剪贴板
- 剪贴板超时自动清除
- `--remember` 保存站点配置；覆盖已记住的不同 username 前会询问确认
- shell completion 生成
- Unix 下 config 目录和文件权限保护

## 安装

### 从源码安装

需要 Rust 工具链。

```bash
# 直接编译安装
cargo install --path . --locked
# 本地构建 release binary，需要自己添加到path
cargo build --release
```

### 从 GitHub Release 安装

可以从这里选择需要的版本下载，https://github.com/0x3ea/keyforge/releases

Linux / macOS 可以使用安装脚本：

```bash
curl -fsSL https://github.com/0x3ea/keyforge/releases/latest/download/install.sh | sh
```

Windows 不提供安装脚本。请从 GitHub Release 手动下载 `keyforge-x86_64-pc-windows-msvc.zip`，解压出 `keyforge.exe`，放到固定目录（如 `%LOCALAPPDATA%\Programs\keyforge\bin\`）并加入用户环境变量 `Path`；重开终端后用 `keyforge --version` 验证。

## 使用

### 生成并打印密码

```bash
keyforge github.com -p
```

程序会提示输入并确认 master password。

### 指定 username

```bash
keyforge github.com -u alice -p
```

username 会被归一化为小写。

### 指定密码长度

```bash
keyforge github.com -u alice -l 20 -p
```

当前生成密码长度范围为 `12..=128`。

### 启用符号

```bash
keyforge github.com -u alice -l 24 -s -p
```

`-s` / `--symbols` 使用内置符号集：

```text
!@#$%^&*-_=+
```

自定义符号集使用等号形式：

```bash
keyforge github.com -u alice -l 24 --symbols="./{}" -p
```

符号集为空（例如站点配置中的 `""`）表示不含符号。

### 复制到剪贴板

不传 `-p` 时，密码会写入剪贴板：

```bash
keyforge github.com -u alice --timeout 45
```

默认超时为 45 秒。超时后，如果剪贴板内容仍然是刚复制的密码，KeyForge 会清空当前剪贴板。

### 记住站点配置

```bash
keyforge github.com -u alice -l 20 -s -r -p
```

`--remember` 会保存当前站点的：

- username
- length
- symbols 符号集

如果该站点已经被记住、但本次传入的 username 与已保存的不同，KeyForge 会先询问是否覆盖（`[y/N]`，默认否），避免误改永久配置；username 相同而仅 length / symbols 变化时则直接更新。

之后可以直接运行：

```bash
keyforge github.com -p
```

KeyForge 会从配置文件读取该站点的规则。

### 生成 shell completion

```bash
keyforge completion bash
keyforge completion zsh
keyforge completion fish
keyforge completion powershell
```

将输出内容安装到对应 shell 的补全目录即可。

## 配置文件

配置文件位置由系统配置目录决定，通常为：

```text
~/.config/keyforge/config.json
```

配置示例：

```json
{
  "defaultUserName": "",
  "defaultLength": 16,
  "defaultSymbols": "!@#$%^&*-_=+",
  "defaultTimeout": 45,
  "defaultPrint": false,
  "defaultRemember": false,
  "sites": {
    "github.com": {
      "userName": "alice",
      "length": 20,
      "symbols": "./{}"
    }
  }
}
```

站点 `symbols` 为三态：字段缺省时继承 `defaultSymbols`；`""` 表示该站显式禁用符号；非空字符串使用该字符串作为自定义符号集。`defaultSymbols` 为空时默认不含符号。

Unix 下 KeyForge 会尽量确保：

- 配置目录权限为 `0700`
- 配置文件权限为 `0600`

配置文件不保存 master password，也不保存生成出的密码，但会保存站点名、username 和密码规则，这些仍然属于敏感元数据。

## Site 归一化规则

KeyForge 会将输入 site 归一化为 hostname 小写形式。

例如：

```text
GitHub.com                  -> github.com
https://GitHub.com/path     -> github.com
  github.com                -> github.com
```

注意：路径不会参与密码派生。因此下面两个输入会对应同一个 site：

```text
https://example.com/login
https://example.com/account
```

它们都会归一化为：

```text
example.com
```

## 派生算法与稳定性

KeyForge 的算法参数通过 domain separator 的版本号锁定，保证相同输入永远得到相同输出。生效字符集、目标长度等派生输入会绑定进 HMAC message，因此不同符号配置也会产生不同密码。

**密钥派生（KDF）**

- 算法：Argon2id（version `0x13`）
- 参数：`m_cost = 65536`（64 MiB）、`t_cost = 3`、`p_cost = 4`、输出 64 字节
- salt 格式：`keyforge-argon2-v1|{site 长度}|{site}|{username 长度}|{username}`，其中 `keyforge-argon2-v1` 是 domain separator，用于把这一层算法版本与其它用途隔离

**密码编码**

- 算法：HMAC-SHA256，domain separator 为 `keyforge-password-encode-v3`
- 生效字符集按固定顺序由小写、大写、数字以及非空符号集拼接而成；完整 charset 与 `length` 都绑定进 HMAC message
- 小写 `a-z`、大写 `A-Z`、数字 `0-9` 始终启用；符号集为空时不含符号
- 内置符号集为 `!@#$%^&*-_=+`（12 个）
- 覆盖保证常开：每个生效类至少出现 1 个字符；不满足时以整段拒绝采样换 round 重试，`MAX_ROUNDS = 4096`
- 单字符采样仍使用拒绝采样，避免 `byte % charset_len` 带来的取模偏差

domain separator 里的版本号用于锁定算法；一旦需要调整编码规则，会递增版本号以区分新旧派生结果，而不是悄悄改变现有站点的输出。

## 未来计划

- Chrome / Edge / Firefox 浏览器插件：在浏览器中识别当前站点，调用 KeyForge 规则生成密码，减少手动输入 site 的步骤。

## 开发

### 项目结构

```text
.
├── Cargo.toml                 # Rust package 配置和依赖
├── Cargo.lock                 # 锁定依赖版本
├── README.md                  # 项目说明
├── scripts/
│   └── install.sh             # Linux / macOS 安装脚本
├── .github/workflows/         # CI / Release workflow
├── tests/
│   └── integration.rs         # 集成测试
└── src/
    ├── lib.rs                 # library crate 模块导出
    ├── main.rs                # CLI 入口和主流程编排
    ├── cli.rs                 # clap 参数定义、子命令和 site 归一化
    ├── config.rs              # JSON 配置读写、默认值和权限处理
    ├── crypto.rs              # salt 构建和 Argon2id 密钥派生
    ├── encode.rs              # HMAC 扩展、拒绝采样和密码编码
    ├── sensitive.rs           # SecretVec，负责 zeroize 和 mlock
    ├── term.rs                # master password 隐藏输入和确认
    ├── clipboard.rs           # 剪贴板写入、超时清除和 Ctrl+C 处理
    └── completions.rs         # shell completion 生成
```

## 使用风险和注意事项

KeyForge 是确定性密码生成器，使用前请理解以下风险。

### 1. 忘记 master password 就无法恢复密码

KeyForge 不保存密码，也没有恢复机制。如果你忘记 master password，过去生成的站点密码无法恢复。

### 2. 输入必须保持一致

生成结果依赖以下输入：

- master password 主密码
- site 站点
- username 用户名
- length 密码长度
- symbols 密码符号集

任意一项变化，都会生成不同密码。

例如：

```text
github.com + alice
```

和：

```text
github.com + alice@example.com
```

会生成不同密码。

### 3. 站点归一化只保留 hostname

KeyForge 会忽略 URL path、query、fragment，只使用 hostname。

这意味着：

```text
https://example.com/login
https://example.com/admin
```

会使用同一个 site：

```text
example.com
```

如果你希望同一域名下不同服务使用不同密码，需要通过不同 username 或不同 site 约定区分。

### 4. username 默认会转为小写

`Alice`、`ALICE`、`alice` 会被视为同一个 username：

```text
alice
```

如果某个服务区分 username 大小写，需要注意这一点。

### 5. `--print` 会把密码暴露在终端

使用：

```bash
keyforge github.com -p
```

会将密码打印到标准输出。风险包括：

- 被旁边的人看到
- 被终端滚动历史保留
- 被录屏或日志系统记录
- 被 shell wrapper 或命令捕获工具记录

如果不是调试或手动复制，优先使用剪贴板模式。

### 6. 剪贴板自动清除不能清除剪贴板历史

KeyForge 可以清除当前系统剪贴板内容，但不能可靠清除剪贴板管理器已经保存的历史记录。

例如以下工具或桌面环境可能保存历史：

- KDE Klipper
- CopyQ
- GNOME clipboard extensions
- cliphist
- 其他剪贴板管理器

如果你使用剪贴板管理器，请将 KeyForge 复制的密码加入忽略规则，或关闭密码相关历史记录。

### 7. 剪贴板内容可能被其他程序读取

很多桌面程序都可以访问剪贴板。复制密码后，在超时清除前，其他本地程序可能读取到密码。

### 8. 配置文件包含敏感元数据

配置文件不包含 master password 和生成出的密码，但包含：

- 使用过的网站
- username
- 每个站点的密码长度和符号策略

这些信息可能暴露你的账户分布和登录习惯。请保护好配置文件和系统账户。

### 9. 版本升级可能改变输出规则

当前项目仍处于早期阶段。未来如果编码算法、site 规则或配置解析规则发生变化，相同输入可能生成不同密码。

目前的 KDF 与编码分别通过 domain separator `keyforge-argon2-v1` 和 `keyforge-password-encode-v3` 锁定版本（详见「派生算法与稳定性」），它们是判断输出是否变化的关键。v3 修改了字符集绑定、默认符号集和覆盖保证，因此从早期版本升级会改变旧输出。

正式使用前，建议：

- 固定使用某个 release 版本
- 记录版本号
- 升级前先验证关键站点输出是否符合预期

KeyForge 只能降低密码存储和派生过程中的部分风险，不能替代系统安全。
