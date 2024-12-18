do $$
declare
    policy_id integer := 1001;
    policy hash28type := decode('6c969320597b755454ff3653ad09725d590c570827a129aeb4385526', 'hex');
    asset_name asset32type := '\x546573744275647a507265766965775f3335';
    validator_addr text := 'addr_test1wrhvtvx3f0g9wv9rx8kfqc60jva3e07nqujk2cspekv4mqs9rjdvz';

    policy2_id integer := 1002;
    policy2 hash28type := decode('aaaabbaa597b755454ff3653ad09725d590c570827a129aeb438ffff', 'hex');
    asset2_name asset32type := '\x656565';
    validator2_addr text := 'addr_test1aaaabbaaf0g9wv9rx8kfqc60jva3e07nqujk2cspekv4mqs9ffff';


    genesis_hash hash32type := decode('b000000000000000000000000000000000000000000000000000000000000000','hex');
    block_hash_1 hash32type := decode('b000000000000000000000000000000000000000000000000000000000000001','hex');
    block_hash_2 hash32type := decode('b000000000000000000000000000000000000000000000000000000000000002','hex');
    block_hash_3 hash32type := decode('b000000000000000000000000000000000000000000000000000000000000003','hex');
    block_hash_4 hash32type := decode('b000000000000000000000000000000000000000000000000000000000000004','hex');
    block_hash_5 hash32type := decode('b000000000000000000000000000000000000000000000000000000000000005','hex');
    block_hash_6 hash32type := decode('b000000000000000000000000000000000000000000000000000000000000006','hex');

    transfer_tx_id_1 integer := 1;
    transfer_tx_id_2 integer := 2;
    irrelevant_tx_id integer := 3;
    transfer_tx_id_3 integer := 4;
    transfer_tx_id_4 integer := 5;
    token2_transfer_tx_id integer := 6;

    transfer_tx_hash_1 hash32type := decode('f000000000000000000000000000000000000000000000000000000000000001','hex');
    transfer_tx_hash_2 hash32type := decode('f000000000000000000000000000000000000000000000000000000000000002','hex');
    irrelevant_tx_hash hash32type := decode('f000000000000000000000000000000000000000000000000000000000000003','hex');
    transfer_tx_hash_3 hash32type := decode('f000000000000000000000000000000000000000000000000000000000000004','hex');
    transfer_tx_hash_4 hash32type := decode('f000000000000000000000000000000000000000000000000000000000000005','hex');
    token2_transer_tx_hash hash32type := decode('f000000000000000000000000000000000000000000000000000000000000006','hex');

    transfer_utxo_id_1   integer := 0;
    transfer_utxo_id_2   integer := 1;
    irrelevant_utxo_id_1 integer := 2;
    irrelevant_utxo_id_2 integer := 3;
    transfer_utxo_id_3   integer := 4;
    transfer_utxo_id_4   integer := 5;
    token2_transfer_utxo_id integer := 6;
begin

insert into multi_asset
(id        , policy                                                      , "name"                                  , fingerprint)
VALUES
(0         , '\xbadbadbadbadbadbadbadbadbadbadbadbadbadbadbadbadbadbad01', '\xbadbadbadbadbadbadbadbadbadbadbad001', 'asset1thisassetshouldbeignoredbythequeries01'),
(policy_id , policy                                                      , asset_name                              , 'asset1yedvsfmkxu27zaaa37lw44pa8ql9favqlyclnm'),
(2         , '\xbadbadbadbadbadbadbadbadbadbadbadbadbadbadbadbadbadbad02', '\xbadbadbadbadbadbadbadbadbadbadbad002', 'asset1thisassetshouldbeignoredbythequeries02'),
(policy2_id, policy2                                                     , asset2_name                             , 'asset1aaaabbaaxu27zaaa37lw44pa8ql9favqlyffff')
;

-- the integration test assume a securityParameter of 1

INSERT INTO block
(id, hash        , epoch_no, slot_no , epoch_slot_no, block_no, previous_id, slot_leader_id, size, "time"                     , tx_count, proto_major, proto_minor, vrf_key, op_cert, op_cert_counter)
VALUES
(0 , genesis_hash, 189     , 189410, 410            , 0       , NULL       , 0             , 1024, '2022-04-21T16:28:00Z'     , 1       , 0          , 0          , ''     , NULL   , NULL           ),
(1 , block_hash_1, 190     , 190400, 400            , 1       , NULL       , 0             , 1024, '2022-04-21T16:44:30Z'     , 1       , 0          , 0          , ''     , NULL   , NULL           ),
(2 , block_hash_2, 190     , 190500, 500            , 2       , NULL       , 0             , 1024, '2022-04-21T16:46:10Z'     , 1       , 0          , 0          , ''     , NULL   , NULL           ),
(3 , block_hash_3, 191     , 191500, 500            , 3       , NULL       , 0             , 1024, '2022-04-21T17:02:50Z'     , 1       , 0          , 0          , ''     , NULL   , NULL           ),
(4 , block_hash_4, 192     , 192500, 500            , 4       , NULL       , 0             , 1024, '2022-04-21T17:19:30Z'     , 1       , 0          , 0          , ''     , NULL   , NULL           ),
(5 , block_hash_5, 193     , 193500, 500            , 5       , NULL       , 0             , 1024, '2022-04-21T17:36:10Z'     , 1       , 0          , 0          , ''     , NULL   , NULL           ),
(6 , block_hash_6, 194     , 194500, 500            , 6       , NULL       , 0             , 1024, '2022-04-21T17:52:50Z'     , 0       , 0          , 0          , ''     , NULL   , NULL           )
;
-- sometimes the block number can be null so we add this block just to handle that case
INSERT INTO block
(id , hash                                                                            , epoch_no, slot_no, epoch_slot_no, block_no, previous_id, slot_leader_id, "size", "time"                   , tx_count, proto_major, proto_minor, vrf_key, op_cert, op_cert_counter)
VALUES
(100, decode('b000000000000000000000000000000000000000000000000000000000000BAD','hex'), NULL    , NULL   , NULL         , NULL    , NULL       , 1             , 0     , '2022-06-06 23:00:00.000', 0       , 0          , 0          , NULL   , NULL   , NULL           )
;


INSERT INTO tx
( id                    , hash                    , block_id, block_index, out_sum, fee, deposit, size, invalid_before, invalid_hereafter, valid_contract, script_size )
VALUES
( transfer_tx_id_1      , transfer_tx_hash_1      , 1       , 0          , 0      , 0  , 0      , 1024, NULL          , NULL             , TRUE          , 1024        ),
( transfer_tx_id_2      , transfer_tx_hash_2      , 3       , 0          , 0      , 0  , 0      , 1024, NULL          , NULL             , TRUE          , 1024        ),
( irrelevant_tx_id      , irrelevant_tx_hash      , 3       , 1          , 0      , 0  , 0      , 1024, NULL          , NULL             , TRUE          , 1024        ),
( transfer_tx_id_3      , transfer_tx_hash_3      , 5       , 0          , 0      , 0  , 0      , 1024, NULL          , NULL             , TRUE          , 1024        ),
( transfer_tx_id_4      , transfer_tx_hash_4      , 5       , 1          , 0      , 0  , 0      , 1024, NULL          , NULL             , TRUE          , 1024        ),
( token2_transfer_tx_id , token2_transer_tx_hash  , 5       , 2          , 0      , 0  , 0      , 1024, NULL          , NULL             , TRUE          , 1024        )
;

INSERT INTO tx_out
( id                      , tx_id                 , index, address         , address_raw, address_has_script, payment_cred, stake_address_id, value, data_hash )
VALUES
( transfer_utxo_id_1      , transfer_tx_id_1      , 0    , validator_addr  , ''         , TRUE              , NULL        , NULL            , 0    , NULL      ),
( transfer_utxo_id_2      , transfer_tx_id_2      , 1    , validator_addr  , ''         , TRUE              , NULL        , NULL            , 0    , NULL      ),
( irrelevant_utxo_id_1    , irrelevant_tx_id      , 0    , 'other_addr'    , ''         , TRUE              , NULL        , NULL            , 0    , NULL      ),
( irrelevant_utxo_id_2    , irrelevant_tx_id      , 1    , 'another_addr'  , ''         , TRUE              , NULL        , NULL            , 0    , NULL      ),
( transfer_utxo_id_3      , transfer_tx_id_3      , 2    , validator_addr  , ''         , TRUE              , NULL        , NULL            , 0    , NULL      ),
( transfer_utxo_id_4      , transfer_tx_id_4      , 0    , validator_addr  , ''         , TRUE              , NULL        , NULL            , 0    , NULL      ),
( token2_transfer_utxo_id , token2_transfer_tx_id , 0    , validator2_addr , ''         , TRUE              , NULL        , NULL            , 0    , NULL      )
;

INSERT INTO ma_tx_out
(id   , quantity , tx_out_id               , ident)
VALUES
(3001 , 11       , transfer_utxo_id_1      ,  policy_id),
(3002 , 12       , transfer_utxo_id_2      ,  policy_id),
(3003 , 13       , transfer_utxo_id_3      ,  policy_id),
(3004 , 14       , transfer_utxo_id_4      ,  policy_id),
(3005 , 100      , irrelevant_utxo_id_1    ,  0),
(3006 , 37       , token2_transfer_utxo_id ,  policy2_id)
;

end $$;
