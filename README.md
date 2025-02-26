# ICP Lottery App

This is a simple lottery application running on the Internet Computer (ICP). It allows users to buy lottery tickets, conduct lottery draws, participate in draws, and query information about tickets and draws.

## Features

#### Function Signature

This function allows users to create lottery draw and it returns draw ID.
Example :
```rust
dfx canister call icp_rust_boilerplate_backend create_lottery_draw
```

This function allows users can purchase a lottery ticket.

Example : 
```rust
dfx canister call icp_rust_boilerplate_backend buy_lottery_ticket '(vec {1; 6;5;8;8;9}, 0)'
```
This function allows users to check the details of a purchased lottery ticket by providing its ID.
If the ticket with the specified ID is found, its details are returned. Otherwise, a NotFound error is returned.

Example :
```rust
dfx canister call icp_rust_boilerplate_backend check_lottery_ticket '(0)'
```

This function conducts a lottery draw by providing the ID of the draw.

Example : 
```rust
dfx canister call icp_rust_boilerplate_backend conduct_lottery_draw '(0)'
```

This function retrieves a list of all the lottery tickets.

Example : 
```rust
dfx canister call icp_rust_boilerplate_backend get_all_lottery_tickets
```
This function retrieves a list of all conducted lottery draws.

Example :
```rust
dfx canister call icp_rust_boilerplate_backend get_all_lottery_draws '()'
```

You can run the following commands to start working on it :

```bash
cd icp-lottery/
dfx help
dfx canister --help
```

## Running the project locally

If you want to test your project locally, you can use the following commands:

```bash
# Starts the replica, running in the background
dfx start --background

# Deploys your canisters to the replica and generates your candid interface
dfx deploy
```

Once the job completes, your application will be available at `http://localhost:4943?canisterId={asset_canister_id}`.
![deployed-canister](https://github.com/user-attachments/assets/a95ccc08-fef6-43f9-a889-f9797b2fb2b0)


