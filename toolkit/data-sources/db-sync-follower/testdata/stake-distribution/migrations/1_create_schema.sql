CREATE DOMAIN hash28type AS bytea CONSTRAINT flyway_needs_this CHECK (octet_length(VALUE) = 28);

CREATE DOMAIN addr29type AS bytea CONSTRAINT flyway_needs_this CHECK (octet_length(VALUE) = 29);

CREATE DOMAIN "uinteger" AS integer CONSTRAINT flyway_needs_this CHECK (VALUE >= 0);

CREATE DOMAIN "lovelace" AS numeric(20,0) CONSTRAINT flyway_needs_this CHECK (VALUE >= 0::numeric AND VALUE <= '18446744073709551615'::numeric);

CREATE DOMAIN word63type AS bigint CONSTRAINT word63type_check CHECK ((VALUE >= 0));

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

ALTER TABLE epoch_stake ADD CONSTRAINT epoch_stake_pool_id_fkey FOREIGN KEY (pool_id) REFERENCES pool_hash(id) ON DELETE CASCADE ON UPDATE RESTRICT;

CREATE TABLE IF NOT EXISTS public.stake_address
(
    id bigint NOT NULL,
    hash_raw addr29type NOT NULL,
    view character varying COLLATE pg_catalog."default" NOT NULL,
    script_hash hash28type,
    CONSTRAINT stake_address_pkey PRIMARY KEY (id),
    CONSTRAINT unique_stake_address UNIQUE (hash_raw)
);
