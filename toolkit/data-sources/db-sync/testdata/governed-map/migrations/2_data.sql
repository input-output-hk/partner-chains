DO $$
DECLARE
  governed_map_entry_1 jsonb := '{
    "list": [
      { "bytes": "6B657931" },
      { "bytes": "11111111111111111111111111111111" }
    ]
  }';
  governed_map_entry_2 jsonb := '{
    "list": [
      { "bytes": "6B657932" },
      { "bytes": "22222222222222222222222222222222" }
    ]
  }';
  governed_map_entry_2a jsonb := '{
    "list": [
      { "bytes": "6B657932" },
      { "bytes": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa" }
    ]
  }';
  governed_map_entry_3 jsonb := '{
    "list": [
      { "bytes": "6B657933" },
      { "bytes": "33333333333333333333333333333333" }
    ]
  }';
  governed_map_entry_3a jsonb := '{
    "list": [
      { "bytes": "6B657933" },
      { "bytes": "44444444444444444444444444444444" }
    ]
  }';
  invalid_datum jsonb := '{ "constructor": 0, "fields": [{ "int": 1 }] }';

  cons_tx_id integer := 0;
  ins_tx_ida integer := 1;
  ins_tx_id  integer := 2;
  del_tx_id  integer := 3;
  ins_tx_id2 integer := 4;
  ups_tx_id  integer := 5;
  script_addr text := 'governed_map_test_address';
  -- these hashes are not really important but putting them in variables help in making the data more readable
  -- block hashes
  bhash_1 hash32type := decode('B702000000000000000000000000000000000000000000000000000000000001','hex');
  bhash_2 hash32type := decode('B702000000000000000000000000000000000000000000000000000000000002','hex');
  bhash_3 hash32type := decode('B702000000000000000000000000000000000000000000000000000000000003','hex');
  bhash_4 hash32type := decode('B702000000000000000000000000000000000000000000000000000000000004','hex');
  bhash_5 hash32type := decode('B702000000000000000000000000000000000000000000000000000000000005','hex');
  bhash_6 hash32type := decode('B702000000000000000000000000000000000000000000000000000000000006','hex');
  bhash_7 hash32type := decode('B702000000000000000000000000000000000000000000000000000000000007','hex');
  bhash_8 hash32type := decode('B702000000000000000000000000000000000000000000000000000000000008','hex');
  -- transaction hashes
  thash_1 hash32type := decode('FFFF000000000000000000000000000000000000000000000000000000000001','hex');
  thash_2 hash32type := decode('FFFF000000000000000000000000000000000000000000000000000000000002','hex');
  thash_3 hash32type := decode('FFFF000000000000000000000000000000000000000000000000000000000003','hex');
  thash_4 hash32type := decode('FFFF000000000000000000000000000000000000000000000000000000000004','hex');
  thash_5 hash32type := decode('FFFF000000000000000000000000000000000000000000000000000000000005','hex');
  thash_6 hash32type := decode('FFFF000000000000000000000000000000000000000000000000000000000006','hex');
  -- data hashes
  dhash_1 hash32type := decode('DA7A000000000000000000000000000000000000000000000000000000000001','hex');
  dhash_2 hash32type := decode('DA7A000000000000000000000000000000000000000000000000000000000002','hex');
  dhash_3 hash32type := decode('DA7A000000000000000000000000000000000000000000000000000000000003','hex');
  dhash_4 hash32type := decode('DA7A000000000000000000000000000000000000000000000000000000000004','hex');
  dhash_5 hash32type := decode('DA7A000000000000000000000000000000000000000000000000000000000005','hex');
  dhash_6 hash32type := decode('DA7A000000000000000000000000000000000000000000000000000000000006','hex');

  governed_map_policy hash28type := decode('500000000000000000000000000000000000434845434b504f494e69', 'hex');

BEGIN

INSERT INTO multi_asset ( id   , policy              , name , fingerprint       )
VALUES                  ( 999  , governed_map_policy , ''   , 'assetGovernedMap')
;

INSERT INTO block  (id, hash   , epoch_no, slot_no , epoch_slot_no, block_no, previous_id, slot_leader_id, size, "time"                 , tx_count, proto_major, proto_minor, vrf_key, op_cert, op_cert_counter)
VALUES             (0 , bhash_1, NULL    , 189410  , NULL         , NULL    , NULL       , 0             , 1024, '2022-04-20T16:28:00Z' , 9       , 0          , 0          , ''     , NULL   , NULL           )
                  ,(1 , bhash_2, 189     , 189410  , 410          , 0       , 0          , 0             , 1024, '2022-04-21T16:28:00Z' , 1       , 0          , 0          , ''     , NULL   , NULL           )
                  ,(2 , bhash_3, 190     , NULL    , NULL         , NULL    , 1          , 0             , 1024, '2022-04-21T16:44:30Z' , 0       , 0          , 0          , ''     , NULL   , NULL           )
                  ,(3 , bhash_4, 190     , 190400  , 400          , 1       , 2          , 0             , 1024, '2022-04-21T16:45:30Z' , 1       , 0          , 0          , ''     , NULL   , NULL           )
                  ,(4 , bhash_5, 190     , 190500  , 500          , 2       , 3          , 0             , 1024, '2022-04-21T16:46:10Z' , 1       , 0          , 0          , ''     , NULL   , NULL           )
                  ,(5 , bhash_6, 191     , NULL    , NULL         , NULL    , 4          , 0             , 1024, '2022-04-21T17:02:50Z' , 0       , 0          , 0          , ''     , NULL   , NULL           )
                  ,(6 , bhash_7, 191     , 191500  , 500          , 3       , 5          , 0             , 1024, '2022-04-21T17:08:50Z' , 1       , 0          , 0          , ''     , NULL   , NULL           )
                  ,(7 , bhash_8, 192     , 192500  , 500          , 4       , 6          , 0             , 1024, '2022-04-21T17:19:30Z' , 1       , 0          , 0          , ''     , NULL   , NULL           )
;

INSERT INTO tx ( id         , hash   , block_id, block_index, out_sum, fee, deposit, size, invalid_before, invalid_hereafter, valid_contract, script_size )
    VALUES     ( cons_tx_id , thash_1, 1       , 0          , 0      , 0  , 0      , 1024, NULL          , NULL             , TRUE          , 1024        )
              ,( ins_tx_ida , thash_2, 1       , 1          , 0      , 0  , 0      , 1024, NULL          , NULL             , TRUE          , 1024        )
              ,( ins_tx_id  , thash_3, 1       , 2          , 0      , 0  , 0      , 1024, NULL          , NULL             , TRUE          , 1024        )
              ,( del_tx_id  , thash_4, 4       , 0          , 0      , 0  , 0      , 1024, NULL          , NULL             , TRUE          , 1024        )
              ,( ins_tx_id2 , thash_5, 6       , 0          , 0      , 0  , 0      , 1024, NULL          , NULL             , TRUE          , 1024        )
              ,( ups_tx_id  , thash_6, 7       , 0          , 0      , 0  , 0      , 1024, NULL          , NULL             , TRUE          , 1024        )
;

INSERT INTO tx_out ( id   , tx_id      , index, address     , address_raw, address_has_script, payment_cred, stake_address_id, value, data_hash )
            VALUES ( 0    , cons_tx_id , 0    , 'other_addr', ''         , TRUE              , NULL        , NULL            , 0    , NULL      )
                  ,( 1    , cons_tx_id , 1    , 'other_addr', ''         , TRUE              , NULL        , NULL            , 0    , NULL      )
                  ,( 2    , cons_tx_id , 2    , 'other_addr', ''         , TRUE              , NULL        , NULL            , 0    , NULL      )
                  ,( 3    , ins_tx_ida , 0    , script_addr , ''         , TRUE              , NULL        , NULL            , 0    , dhash_1   ) -- add key2a
                  ,( 4    , ins_tx_id  , 0    , script_addr , ''         , TRUE              , NULL        , NULL            , 0    , dhash_2   ) -- add key1
                  ,( 5    , ins_tx_id  , 1    , script_addr , ''         , TRUE              , NULL        , NULL            , 0    , dhash_3   ) -- add invalid
                  ,( 7    , ins_tx_id  , 2    , script_addr , ''         , TRUE              , NULL        , NULL            , 0    , dhash_4   ) -- add key2 simulating 2 utxos with the same key
                  ,( 8    , del_tx_id  , 0    , 'other_addr', ''         , TRUE              , NULL        , NULL            , 0    , NULL      ) -- delete key1
                  ,( 9    , ins_tx_id2 , 0    , script_addr , ''         , TRUE              , NULL        , NULL            , 0    , dhash_5   ) -- add key3
                  ,( 10   , ups_tx_id  , 0    , script_addr , ''         , TRUE              , NULL        , NULL            , 0    , dhash_6   ) -- upsert key3
;

INSERT INTO ma_tx_out ( id   , quantity , tx_out_id , ident)
VALUES                ( 0    , 1        , 3         , 999  )
                     ,( 1    , 1        , 4         , 999  )
                     ,( 2    , 1        , 5         , 999  )
                     ,( 3    , 1        , 7         , 999  )
                     ,( 4    , 1        , 8         , 999  )
                     ,( 5    , 1        , 9         , 999  )
                     ,( 6    , 1        , 10        , 999  )
;

INSERT INTO tx_in ( id, tx_in_id   , tx_out_id  , tx_out_index, redeemer_id )
           VALUES ( 0 , ins_tx_id  , cons_tx_id , 0           , NULL        )
                 ,( 1 , ins_tx_id  , cons_tx_id , 1           , NULL        )
                 ,( 2 , ins_tx_id2 , cons_tx_id , 0           , NULL        )
                 ,( 4 , del_tx_id  , ins_tx_id  , 0           , NULL        )
                 ,( 5 , del_tx_id  , ins_tx_id  , 1           , NULL        )
                 ,( 6 , ups_tx_id  , ins_tx_id2 , 0           , NULL        )
;

INSERT INTO datum ( id   , hash    , tx_id     , value                )
           VALUES ( 1    , dhash_1 , ins_tx_ida, governed_map_entry_2a)
                 ,( 2    , dhash_2 , ins_tx_id , governed_map_entry_1 )
                 ,( 3    , dhash_3 , ins_tx_id , invalid_datum        )
                 ,( 4    , dhash_4 , ins_tx_id , governed_map_entry_2 )
                 ,( 5    , dhash_5 , ins_tx_id2, governed_map_entry_3 )
                 ,( 6    , dhash_6 , ups_tx_id , governed_map_entry_3a)
;

END $$;
