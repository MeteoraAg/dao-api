# dao-api
Service to provide common information for Meteora incentives system

# Build
cargo build

# Usage (from keeper folder)
../target/debug/dao-keeper --base ba1AznDonanrFY2Ek6jaiMmkccMeU43A5TXU2jB8f4N --socket-address https://api.devnet.solana.com --postgres-user mercurial --postgres-password mercurial1234 --postgres-db keeper --postgres-socket-address localhost:5432 --provider https://api.devnet.solana.com --should-crank 1