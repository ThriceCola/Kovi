**English** | [简体中文](https://thricecola.github.io/kovi-doc/)

# Kovi

Kovi is a simple and extensible chat bot development framework. If you want to develop Milky / OneBot V11 bots using Rust, Kovi is a great choice.

🎯 The goal is to create the simplest chat bot framework in Rust! Simplifying complex Rust syntax? Kovi has done its best.

🤔 Let me count — the quick start in the documentation only requires 10 lines of code to create the simplest plugin.

🥁 There's also a CLI tool to make project development easier.

🖥️ Just for fun? Or customized? Kovi is competent

🛍️ The plugin shop provides an excellent Kovi shopping experience, allowing you to easily access packages from plugin developers 📦.

😍 The project documentation is very simple and easy to understand. Follow it and you'll be good to go.

### ↓ Documentation is here

[Kovi Docs](https://thricecola.github.io/kovi-doc/)

### ↓ The shop is here

[Kovi Shop](https://thricecola.github.io/kovi-doc/start/plugins.html)


## Protocol Support

Kovi is an "event bus" plugin runner. It connects to chat services through **driver crates** that implement different protocols.

- `kovi-milky` — Milky WebSocket protocol
- `kovi-onebot` — OneBot V11 forward WebSocket protocol

When creating a project, `kovi-cli` will ask you to choose a driver, or you can specify it with `--driver milky` / `--driver onebot`.

## Getting Started

It's recommended to use `kovi-cli` to manage your Kovi bot project.

```bash
cargo install kovi-cli
```

1. Create a basic Rust project and add the framework.

```bash
cargo kovi new my-kovi-bot
cd ./my-kovi-bot
```

During this step, you'll be prompted to choose a protocol driver and whether to add the command plugin.

```
✔ Which driver/protocol to use? · Milky
✔ Are you want to add message command plugins? · Yes
```

You can also skip the prompts by passing flags:

```bash
cargo kovi n my-kovi-bot --driver onebot --cmd
```

2. A bot instance has been generated in **src/main.rs**.

```rust
use kovi::tokio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let driver_config = kovi_milky::load_local_conf()?;
    let driver = kovi_milky::MilkyDriver::new(driver_config);

    let bot = kovi::build_bot!(driver;);

    bot.run().await;
    Ok(())
}
```

On your first run, during `driver::load_local_conf()`, you'll be prompted to enter some information to create the `kovi.conf.toml` file, which is required for the driver to run.

```
✔ What is the type of the host of the OneBot server? · IPv4

✔ What is the IP of the OneBot server? · 127.0.0.1
(Default: 127.0.0.1)

✔ What is the port of the OneBot server? · 8081
(Default: 8081)

✔ What is the access_token of the OneBot server? (Optional) ·
(Default: empty)

✔ What is the ID of the main administrator? (Not used yet)
(Optional)

✔ Do you want to view more optional options? · No
```

## Plugin Development

### Creating a Plugin

Follow the steps below.

```bash
cargo kovi create hi
```

`kovi-cli` and `cargo` will take care of everything for you. The CLI will automatically detect which driver crates are in your workspace and add the corresponding `use` imports.

You will see that a new `plugins/hi` directory has been created. This is also the recommended way to develop plugins, as it's always good to manage them in a directory.

### Writing a Plugin

Edit your newly created plugin in `plugins/hi/src/lib.rs`.

Here's a minimal example:

```rust
// Import the plugin builder structure
use kovi::PluginBuilder as plugin;
// Import the driver traits (kovi-milky / kovi-onebot)
use kovi_milky::*;

#[kovi::plugin] // Build the plugin
async fn main() {
    plugin::on_msg(|event| async move {
        // on_msg() listens for messages, and event contains all the information of the current message.
        if event.borrow_text() == Some("Hi Bot") {
            event.reply("Hi!") // Quick reply
        }
    });
}
```

The main function is written in `lib.rs` because it will be exported later to be mounted to the bot instance.

Plugins generally don't need a `main.rs`.

### Mounting the Plugin

```bash
cargo kovi add hi
```

Alternatively, you can use `cargo` directly; both are the same. This will add a local dependency in the root project's `Cargo.toml`.

```bash
cargo add --path plugins/hi
```

Then mount the plugin in `src/main.rs`:

```rust
use kovi::tokio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let driver_config = kovi_milky::load_local_conf()?;
    let driver = kovi_milky::MilkyDriver::new(driver_config);

    let bot = kovi::build_bot!(driver; hi, hi2, plugin123);

    bot.run().await;
    Ok(())
}
```

### More Plugin Examples

#### Bot Taking Initiative to Send Messages

```rust
use kovi::PluginBuilder as plugin;
use kovi_milky::*;

#[kovi::plugin]
async fn main() {
    // get a RuntimeBot
    let bot = plugin::get_runtime_bot();
    let user_id = bot.main_admin;

    bot.send_private_msg(user_id, "bot online")
}
```

The `main()` function runs only once when plugin starts.

The closure passed to `plugin::on_msg()` runs every time a message is received.

Kovi has encapsulated all available OneBot standard APIs. To extend the API, you can use `RuntimeBot`'s `send_api()` to send APIs yourself. You can check out the API extension plugins available for your needs at [Kovi Plugin Shop](https://thricecola.github.io/kovi-doc/start/plugins.html).

You can find more documentation in the [Kovi Doc](https://thricecola.github.io/kovi-doc/).
