CREATE DOMAIN hash28type AS bytea CONSTRAINT flyway_needs_this CHECK (octet_length(VALUE) = 28);

CREATE DOMAIN hash32type AS bytea CONSTRAINT flyway_needs_this CHECK(octet_length(VALUE) = 32);

CREATE DOMAIN "uinteger" AS integer CONSTRAINT flyway_needs_this CHECK (VALUE >= 0);

CREATE DOMAIN "lovelace" AS numeric(20,0) CONSTRAINT flyway_needs_this CHECK (VALUE >= 0::numeric AND VALUE <= '18446744073709551615'::numeric);

CREATE DOMAIN "word64type" AS numeric(20,0)	CONSTRAINT flyway_needs_this CHECK (VALUE >= 0::numeric AND VALUE <= '18446744073709551615'::numeric);

CREATE DOMAIN word63type AS bigint CONSTRAINT word63type_check CHECK ((VALUE >= 0));

CREATE DOMAIN word31type AS integer CONSTRAINT word31type_check CHECK ((VALUE >= 0));

CREATE DOMAIN txindex AS smallint CONSTRAINT txindex_check CHECK ((VALUE >= 0));

CREATE DOMAIN asset32type AS bytea CHECK (octet_length (VALUE) <= 32);

CREATE DOMAIN int65type AS numeric (20, 0) CHECK (VALUE >= -18446744073709551615 AND VALUE <= 18446744073709551615);

CREATE TYPE scriptpurposetype AS ENUM ('spend', 'mint', 'cert', 'reward');

CREATE TABLE epoch_param (
	id bigserial NOT NULL,
	epoch_no "uinteger" NOT NULL,
	min_fee_a "uinteger" NOT NULL,
	min_fee_b "uinteger" NOT NULL,
	max_block_size "uinteger" NOT NULL,
	max_tx_size "uinteger" NOT NULL,
	max_bh_size "uinteger" NOT NULL,
	key_deposit "lovelace" NOT NULL,
	pool_deposit "lovelace" NOT NULL,
	max_epoch "uinteger" NOT NULL,
	optimal_pool_count "uinteger" NOT NULL,
	influence float8 NOT NULL,
	monetary_expand_rate float8 NOT NULL,
	treasury_growth_rate float8 NOT NULL,
	decentralisation float8 NOT NULL,
	entropy hash32type NULL,
	protocol_major "uinteger" NOT NULL,
	protocol_minor "uinteger" NOT NULL,
	min_utxo_value "lovelace" NOT NULL,
	min_pool_cost "lovelace" NOT NULL,
	nonce hash32type NULL,
	coins_per_utxo_word "lovelace" NULL,
	cost_model_id int8 NULL,
	price_mem float8 NULL,
	price_step float8 NULL,
	max_tx_ex_mem "word64type" NULL,
	max_tx_ex_steps "word64type" NULL,
	max_block_ex_mem "word64type" NULL,
	max_block_ex_steps "word64type" NULL,
	max_val_size "word64type" NULL,
	collateral_percent "uinteger" NULL,
	max_collateral_inputs "uinteger" NULL,
	block_id int8 NOT NULL,
	CONSTRAINT epoch_param_pkey PRIMARY KEY (id),
	CONSTRAINT unique_epoch_param UNIQUE (epoch_no, block_id)
);
CREATE INDEX idx_epoch_param_block_id ON epoch_param USING btree (block_id);

CREATE TABLE pool_hash (
	id bigserial NOT NULL,
	hash_raw hash28type NOT NULL,
	"view" varchar NOT NULL,
	CONSTRAINT pool_hash_pkey PRIMARY KEY (id),
	CONSTRAINT unique_pool_hash UNIQUE (hash_raw)
);


CREATE TABLE epoch_stake (
	id SERIAL PRIMARY KEY,
	addr_id int8 NOT NULL,
	pool_id int8 NOT NULL,
	amount "lovelace" NOT NULL,
	epoch_no "uinteger" NOT NULL,
	CONSTRAINT unique_stake UNIQUE (epoch_no, addr_id, pool_id)
);
CREATE INDEX idx_epoch_stake_addr_id ON epoch_stake USING btree (addr_id);
CREATE INDEX idx_epoch_stake_epoch_no ON epoch_stake USING btree (epoch_no);
CREATE INDEX idx_epoch_stake_pool_id ON epoch_stake USING btree (pool_id);

CREATE TABLE block (
    id bigint NOT NULL,
    hash hash32type NOT NULL,
    epoch_no word31type,
    slot_no word63type,
    epoch_slot_no word31type,
    block_no word31type,
    previous_id bigint,
    slot_leader_id bigint NOT NULL,
    size word31type NOT NULL,
    "time" timestamp without time zone NOT NULL,
    tx_count bigint NOT NULL,
    proto_major word31type NOT NULL,
    proto_minor word31type NOT NULL,
    vrf_key character varying,
    op_cert hash32type,
    op_cert_counter word63type,
    CONSTRAINT block_pkey PRIMARY KEY (id)
);

CREATE TABLE datum (
    id bigint NOT NULL,
    hash hash32type NOT NULL,
    tx_id bigint NOT NULL,
    value JSONB,
    CONSTRAINT datum_id PRIMARY KEY (id)
);

CREATE TABLE tx (
    id bigint NOT NULL,
    hash hash32type NOT NULL,
    block_id bigint NOT NULL,
    block_index word31type NOT NULL,
    out_sum lovelace NOT NULL,
    fee lovelace NOT NULL,
    deposit bigint NOT NULL,
    size word31type NOT NULL,
    invalid_before word64type,
    invalid_hereafter word64type,
    valid_contract boolean NOT NULL,
    script_size word31type NOT NULL,
    CONSTRAINT tx_pkey PRIMARY KEY (id),
     -- this constraint unique_tx_block_index does not exist in the real database but it ensure we put
     -- consistent data in our database
	  CONSTRAINT unique_tx_block_index UNIQUE (block_id, block_index)
);

CREATE TABLE redeemer_data (
	id bigserial NOT NULL,
	hash public.hash32type NOT NULL,
	tx_id int8 NOT NULL,
	value jsonb NULL,
	CONSTRAINT redeemer_data_pkey PRIMARY KEY (id),
	CONSTRAINT unique_redeemer_data UNIQUE (hash)
);
CREATE INDEX redeemer_data_tx_id_idx ON public.redeemer_data USING btree (tx_id);

ALTER TABLE public.redeemer_data ADD CONSTRAINT redeemer_data_tx_id_fkey FOREIGN KEY (tx_id) REFERENCES tx(id) ON DELETE CASCADE ON UPDATE RESTRICT;

CREATE TABLE tx_in (
    id bigint NOT NULL,
    tx_in_id bigint NOT NULL,
    tx_out_id bigint NOT NULL,
    tx_out_index txindex NOT NULL,
    redeemer_id bigint,
    CONSTRAINT tx_in_pkey PRIMARY KEY (id)
);

CREATE TABLE tx_out (
    id bigint NOT NULL,
    tx_id bigint NOT NULL,
    index txindex NOT NULL,
    address character varying NOT NULL,
    address_raw bytea NOT NULL,
    address_has_script boolean NOT NULL,
    payment_cred hash28type,
    stake_address_id bigint,
    value lovelace NOT NULL,
    data_hash hash32type
);

CREATE TABLE redeemer (
    id bigint PRIMARY KEY,
    tx_id bigint NOT NULL,
    unit_mem word63type NOT NULL,
    unit_steps word63type NOT NULL,
    fee lovelace NOT NULL,
    purpose scriptpurposetype NOT NULL,
    index uinteger NOT NULL,
    script_hash hash28type NULL,
    redeemer_data_id bigint NOT NULL
);
ALTER TABLE redeemer ADD CONSTRAINT unique_redeemer UNIQUE(tx_id, purpose, index);
ALTER TABLE redeemer ADD CONSTRAINT redeemer_tx_id_fkey FOREIGN KEY(tx_id) REFERENCES tx(id) ON DELETE CASCADE ON UPDATE RESTRICT;
ALTER TABLE redeemer ADD CONSTRAINT redeemer_datum_id_fkey FOREIGN KEY(redeemer_data_id) REFERENCES redeemer_data(id) ON DELETE CASCADE ON UPDATE RESTRICT;

CREATE TABLE multi_asset (
    id bigint PRIMARY KEY,
    policy hash28type NOT NULL,
    name asset32type NOT NULL,
    fingerprint varchar NOT NULL
);
ALTER TABLE multi_asset ADD CONSTRAINT unique_multi_asset UNIQUE (policy, name);

CREATE TABLE ma_tx_mint (
    id bigint PRIMARY KEY,
    quantity int65type NOT NULL,
    tx_id bigint NOT NULL,
    ident bigint NOT NULL
);
ALTER TABLE ma_tx_mint ADD CONSTRAINT unique_ma_tx_mint UNIQUE(ident, tx_id);
CREATE UNIQUE INDEX idx_ma_tx_mint_tx_id ON ma_tx_mint(tx_id);

ALTER TABLE epoch_stake ADD CONSTRAINT epoch_stake_pool_id_fkey FOREIGN KEY (pool_id) REFERENCES pool_hash(id) ON DELETE CASCADE ON UPDATE RESTRICT;
ALTER TABLE ONLY tx
    ADD CONSTRAINT unique_tx UNIQUE (hash);
ALTER TABLE ONLY tx
    ADD CONSTRAINT tx_block_id_fkey FOREIGN KEY (block_id) REFERENCES block(id) ON UPDATE RESTRICT ON DELETE CASCADE;

CREATE TABLE public.ma_tx_out (
	id bigserial NOT NULL,
	quantity public."word64type" NOT NULL,
	tx_out_id int8 NOT NULL,
	ident int8 NOT NULL,
	CONSTRAINT ma_tx_out_pkey PRIMARY KEY (id),
	CONSTRAINT unique_ma_tx_out UNIQUE (ident, tx_out_id),
	CONSTRAINT ma_tx_out_ident_fkey FOREIGN KEY (ident) REFERENCES public.multi_asset(id) ON DELETE CASCADE ON UPDATE RESTRICT
	-- CONSTRAINT ma_tx_out_tx_out_id_fkey FOREIGN KEY (tx_out_id) REFERENCES public.tx_out(id) ON DELETE CASCADE ON UPDATE RESTRICT
);
CREATE INDEX idx_ma_tx_out_tx_out_id ON public.ma_tx_out USING btree (tx_out_id);
