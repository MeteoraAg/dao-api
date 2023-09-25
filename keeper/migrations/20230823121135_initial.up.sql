-- Add up migration script here
CREATE TABLE IF NOT EXISTS public.token (
    address VARCHAR NOT NULL,
    coingecko_id VARCHAR NOT NULL,
    decimals integer NOT NULL,
    symbol VARCHAR NOT NULL,
    CONSTRAINT token_pk PRIMARY KEY(address)
);


-- CREATE TABLE IF NOT EXISTS public.gauge_factory (
--     address VARCHAR PRIMARY KEY,
--     base VARCHAR NOT NULL,
--     rewarder VARCHAR NOT NULL,
--     locker VARCHAR NOT NULL,
--     foreman VARCHAR NOT NULL,
--     epoch_duration_seconds integer NOT NULL,
--     current_voting_epoch integer NOT NULL,
--     next_epoch_starts_at integer NOT NULL,
-- );
CREATE TABLE IF NOT EXISTS public.crawl_config ( 
    voting_epoch_down BIGINT NOT NULL,
    voting_epoch_up BIGINT NOT NULL
);

-- CREATE TABLE IF NOT EXISTS public.gauge (
--     address VARCHAR PRIMARY KEY,
--     quarry VARCHAR NOT NULL,
--     amm_pool VARCHAR NOT NULL,
--     token_a_fee_key VARCHAR NOT NULL,
--     token_b_fee_key VARCHAR NOT NULL,
--     is_disabled bool NOT NULL,
--     cummulative_token_a_fee BIGINT NOT NULL,
--     cummulative_token_b_fee BIGINT NOT NULL,
--     cummulative_claimed_token_a_fee BIGINT NOT NULL,
--     cummulative_claimed_token_b_fee BIGINT NOT NULL,
--     amm_type integer NOT NULL
-- );


CREATE TABLE IF NOT EXISTS public.epoch_gauge (
    address VARCHAR PRIMARY KEY,
    gauge VARCHAR NOT NULL,
    voting_epoch BIGINT NOT NULL,
    total_power VARCHAR NOT NULL,
    token_a_fee VARCHAR NOT NULL,
    token_b_fee VARCHAR NOT NULL
);

CREATE TABLE IF NOT EXISTS public.bribe (
    address VARCHAR PRIMARY KEY,
    gauge VARCHAR NOT NULL,
    token_mint VARCHAR NOT NULL,
    reward_each_epoch VARCHAR NOT NULL,
    briber VARCHAR NOT NULL,
    token_account_vault VARCHAR NOT NULL,
    bribe_rewards_epoch_start BIGINT NOT NULL,
    bribe_rewards_epoch_end BIGINT NOT NULL,
    bribe_index BIGINT NOT NULL
);