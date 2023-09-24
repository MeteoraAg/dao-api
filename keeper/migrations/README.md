# Migration SQLs for dao-api

## Install

```
 cargo install sqlx-cli
```

## Create migration

```
sqlx migrate add <name>
```

You do not need to run `sqlx migrate run` as the migration will be automatically ran by the keeper at the bootup.

## More information for sqlx

https://github.com/launchbadge/sqlx/tree/main/sqlx-cli
