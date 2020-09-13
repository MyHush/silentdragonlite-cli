# SilentDragonLite CLI

<p align="left">
    <a href="https://twitter.com/MyHushTeam">
        <img src="https://img.shields.io/twitter/url?style=social&url=https%3A%2F%2Ftwitter.com%2Fmyhushteam"
            alt="MyHushTeam's Twitter"></a>
    <a href="https://twitter.com/intent/follow?screen_name=MyHushTeam">
        <img src="https://img.shields.io/twitter/follow/MyHushTeam?style=social&logo=twitter"
            alt="follow on Twitter"></a>
    <a href="https://fosstodon.org/@myhushteam">
        <img src="https://img.shields.io/badge/Mastodon-MyHushTeam-blue"
            alt="follow on Mastodon"></a>
    <a href="https://www.reddit.com/r/Myhush/">
        <img src="https://img.shields.io/reddit/subreddit-subscribers/Myhush?style=social"
            alt="MyHushTeam's Reddit"></a>
</p>

`silentdragonlite-cli` is a command line SilentDragonLite light client. To use it, download the latest binary from the releases page and run `./silentdragonlite-cli`

This will launch the interactive prompt. Type `help` to get a list of commands

## Running in non-interactive mode:
You can also run `silentdragonlite-cli` in non-interactive mode by passing the command you want to run as an argument. For example, `silentdragonlite-cli addresses` will list all wallet addresses and exit. 
Run `silentdragonlite-cli help` to see a list of all commands. 

## Privacy 
* While all the keys and transaction detection happens on the client, the server can learn what blocks contain your shielded transactions.
* The server also learns other metadata about you like your ip address etc...
* Also remember that t-addresses don't provide any privacy protection.

## Notes:
* If you want to run your own server, please see [SilentDragonLite-cli lightwalletd](https://github.com/MyHush/lightwalletd), and then run `./silentdragonlite-cli --server http://127.0.0.1:9069`. You might also need to pass `--dangerous` if you are using a self-signed  TLS certificate.

* The log file is in `~/.silentdragonlite/silentdragonlite-cli.debug.log`. Wallet is stored in `~/.silentdragonlite/silentdragonlite-cli.dat`

### Note Management
silentdragonlite does automatic note and utxo management, which means it doesn't allow you to manually select which address to send outgoing transactions from. It follows these principles:
* Defaults to sending shielded transactions, even if you're sending to a transparent address
* Sapling funds need at least 2 confirmations before they can be spent
* Can select funds from multiple shielded addresses in the same transaction
* Will automatically shield your transparent funds at the first opportunity
    * When sending an outgoing transaction to a shielded address, silentdragonlite can decide to use the transaction to additionally shield your transparent funds (i.e., send your transparent funds to your own shielded address in the same transaction)

## Compiling from source

#### Pre-requisites
* Rust v1.37 or higher.
    * Run `rustup update` to get the latest version of Rust if you already have it installed

```
git clone https://github.com/MyHush/silentdragonlite-cli.git
cargo build --release
./target/release/silentdragonlite-cli
```

## Options
Here are some CLI arguments you can pass to `silentdragonlite-cli`. Please run `silentdragonlite-cli --help` for the full list. 

* `--server`: Connect to a custom SilentDragonLite lightwalletd server. 
    * Example: `./silentdragonlite-cli --server 127.0.0.1:9069`
* `--seed`: Restore a wallet from a seed phrase. Note that this will fail if there is an existing wallet. Delete (or move) any existing wallet to restore from the 24-word seed phrase
    * Example: `./silentdragonlite-cli --seed "twenty four words seed phrase"`
 * `--recover`: Attempt to recover the seed phrase from a corrupted wallet
 
### Support

For support or other questions, join us on [Discord](https://myhush.org/discord), or tweet at [@MyHushTeam](https://twitter.com/MyHushTeam), or toot at our [Mastodon](https://fosstodon.org/@myhushteam) or join [Telegram](http://myhush.org/telegram) or [file an issue](https://github.com/MyHush/silentdragonlite-cli/issues).
