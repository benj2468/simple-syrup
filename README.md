# CryptoPass

> Secure is a process, not a product - Bruce Schneier

## Computer Science 98: 22W

## Structure

- `./derive` - proc_macro derive crate for deriving authenticating server scopes
- `./migrations` - db migrations
- `./src` - server descriptions and root server setup

## Running the Server Locally

- Install [rustup](https://sourabhbajaj.com/mac-setup/Rust/), and [psql](https://formulae.brew.sh/formula/postgresql)
- Run `cd simple-syrup && make prepare`
- Ask Benjamin for a Sendgrid API key, and add it either to your shell profile file or to the .env file. Set the key to `SENDGRID_KEY` add it to the `.env` file.
- `make local`

For reference on homebrew see [here](https://brew.sh/)

## Contributors

- Benjamin Cape

- Professor: Sebastiaan Joosten
