## Gotify2Matrix

A small helper to relay messages from a gotify server to an end2end-encrypted matrix chat.
`gotify2matrix` will persist the session as well as the last synced gotify message id and will continue syncing when it was not running.

## Usage
```
gotify2matrix

USAGE:
    gotify2matrix [OPTIONS] [SUBCOMMAND]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -c, --config <config-file>

SUBCOMMANDS:
    help      Prints this message or the help of the given subcommand(s)
    verify    Wait for incoming device verifications
```

```bash
cargo run --release
```
where config.toml is the config file. See `config.sample.toml`.
Default values are commented out.

## Configuration

### Matrix
| Variable      | Meaning                                                                  | Default Value |
| ------------- | -------------                                                            | ------------- |
| homeserver    | matrix homeserver, e.g. `"https://synapse.someserver.com"`               | N/A           |
| username      | username of the bot                                                      | N/A           |
| password      | password of the bot                                                      | N/A           |
| room_id       | room id of the chat (can be found using Element -> settings -> advanced) | N/A           |
| session_dir   | storage location for the persistent session                              | `"./session"` |

### Gotify
| Variable      | Meaning                                            | Default Value                                        |
| ------------- | -------------                                      | -------------                                        |
| url           | url of the gotify server                           | N/A                                                  |
| token         | app token for the bot                              | N/A                                                  |
| delete_sent   | wether sent messages should be removed from gotify | `false`                                              |
| format_plain  | format string for the plain part                   | `"{{title}} ({{app}}) \n{{message}}"`                |
| format_html   | format string of the html part                     | `"<h4>{{title}} (<u>{{app}}</u>)</h4>\n{{message}}"`

Available template tokens are `title, app, message`.

Instead of a supplied config, all values can also be set using environtmen variables.
Matrix variables are prefixed with `G2M_MATRIX_`, e.g. `G2M_MATRIX_HOMESERVER`, while gotify variable are prefixed with `G2M_GOTIFY_`.

## Docker
Modify `.g2m.sample.env`, save it as `.g2m.env` and run `docker compose up -d` to run the server.

## Verification
After the first run, the new session can be verified using another verified instance.
Start the verification from a another instance and run `cargo run --release -- verify` or `docker compose run gotify2matrix gotify2matrix verify`.
After successful verification, simply quit the program using `CTRL-C`.
