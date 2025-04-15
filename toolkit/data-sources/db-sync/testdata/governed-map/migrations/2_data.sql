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

  ins_tx_id integer       := 0;
  ins_tx_ida integer      := 1;
  del_tx_id integer       := 2;
  consumed_tx_id integer  := 4;
  ins_tx_id2 integer      := 5;
  ups_tx_id integer       := 6;
  script_addr text    := 'governed_map_test_address';
  -- those hashes are not really important but putting them in variables help to make the data more readable
  bhash_0 hash32type := decode('000000000067F14D6F6436C7F7DEDB27CE3CEB4D2D18FF249D43B22D86FAE3F1','hex');
  bhash_1 hash32type := decode('0BEED7FB0067F14D6F6436C7F7DEDB27CE3CEB4D2D18FF249D43B22D86FAE3F1','hex');
  bhash_2 hash32type := decode('000000000167F14D6F6436C7F7DEDB27CE3CEB4D2D18FF249D43B22D86FAE3F1','hex');
  bhash_3 hash32type := decode('ABEED7FB0067F14D6F6436C7F7DEDB27CE3CEB4D2D18FF249D43B22D86FAE3F1','hex');
  bhash_4 hash32type := decode('BBEED7FB0067F14D6F6436C7F7DEDB27CE3CEB4D2D18FF249D43B22D86FAE3F1','hex');
  bhash_5 hash32type := decode('000000000267F14D6F6436C7F7DEDB27CE3CEB4D2D18FF249D43B22D86FAE3F1','hex');
  bhash_6 hash32type := decode('CBEED7FB0067F14D6F6436C7F7DEDB27CE3CEB4D2D18FF249D43B22D86FAE3F1','hex');
  bhash_7 hash32type := decode('DBEED7FB0067F14D6F6436C7F7DEDB27CE3CEB4D2D18FF249D43B22D86FAE3F1','hex');
  hash1 hash32type := decode('ABEED7FB0067F14D6F6436C7F7DEDB27CE3CEB4D2D18FF249D43B22D86FAE3F1','hex');
  hash2 hash32type := decode('BBEED7FB0067F14D6F6436C7F7DEDB27CE3CEB4D2D18FF249D43B22D86FAE3F1','hex');
  hash3 hash32type := decode('CBEED7FB0067F14D6F6436C7F7DEDB27CE3CEB4D2D18FF249D43B22D86FAE3F1','hex');
  hash4 hash32type := decode('DBEED7FB0067F14D6F6436C7F7DEDB27CE3CEB4D2D18FF249D43B22D86FAE3F1','hex');
  hash5 hash32type := decode('055557FB0067F14D6F6436C7F7DEDB27CE3CEB4D2D18FF249D43B22D86FAE3F1','hex');
  hash6 hash32type := decode('01EED7FB0067F14D6F6436C7F7DEDB27CE3CEB4D2D18FF249D43B22D86FAE3F1','hex');
  hash7 hash32type := decode('5000000000000000000000000000000000000000000000000000000000000009', 'hex');
  consumed_tx_hash  hash32type := decode('cdefe62b0a0016c2ccf8124d7dda71f6865283667850cc7b471f761d2bc1eb13','hex');

  governed_map_policy hash28type := decode('500000000000000000000000000000000000434845434b504f494e69', 'hex');

BEGIN

INSERT INTO multi_asset ( id   , policy              , name , fingerprint       )
VALUES                  ( 999  , governed_map_policy , ''   , 'assetGovernedMap')
;

INSERT INTO block  (id, hash   , epoch_no, slot_no , epoch_slot_no, block_no, previous_id, slot_leader_id, size, "time"                 , tx_count, proto_major, proto_minor, vrf_key, op_cert, op_cert_counter)
VALUES             (0 , bhash_0, NULL    , 189410  , NULL         , NULL    , NULL       , 0             , 1024, '2022-04-20T16:28:00Z' , 9       , 0          , 0          , ''     , NULL   , NULL           )
                  ,(1 , bhash_1, 189     , 189410  , 410          , 0       , 0          , 0             , 1024, '2022-04-21T16:28:00Z' , 1       , 0          , 0          , ''     , NULL   , NULL           )
                  ,(2 , bhash_2, 190     , NULL    , NULL         , NULL    , 1          , 0             , 1024, '2022-04-21T16:44:30Z' , 0       , 0          , 0          , ''     , NULL   , NULL           )
                  ,(3 , bhash_3, 190     , 190400  , 400          , 1       , 2          , 0             , 1024, '2022-04-21T16:44:30Z' , 1       , 0          , 0          , ''     , NULL   , NULL           )
                  ,(4 , bhash_4, 190     , 190500  , 500          , 2       , 3          , 0             , 1024, '2022-04-21T16:46:10Z' , 1       , 0          , 0          , ''     , NULL   , NULL           )
                  ,(5 , bhash_5, 191     , NULL    , NULL         , NULL    , 4          , 0             , 1024, '2022-04-21T17:02:50Z' , 0       , 0          , 0          , ''     , NULL   , NULL           )
                  ,(6 , bhash_6, 191     , 191500  , 500          , 3       , 5          , 0             , 1024, '2022-04-21T17:02:50Z' , 1       , 0          , 0          , ''     , NULL   , NULL           )
                  ,(7 , bhash_7, 192     , 192500  , 500          , 4       , 6          , 0             , 1024, '2022-04-21T17:19:30Z' , 1       , 0          , 0          , ''     , NULL   , NULL           )
;

INSERT INTO tx ( id            , hash            , block_id, block_index, out_sum, fee, deposit, size, invalid_before, invalid_hereafter, valid_contract, script_size )
    VALUES     ( consumed_tx_id, consumed_tx_hash, 1       , 0          , 0      , 0  , 0      , 1024, NULL          , NULL             , TRUE          , 1024        )
              ,( ins_tx_id     , hash1           , 1       , 1          , 0      , 0  , 0      , 1024, NULL          , NULL             , TRUE          , 1024        )
              ,( ins_tx_ida    , hash2           , 1       , 2          , 0      , 0  , 0      , 1024, NULL          , NULL             , TRUE          , 1024        )
              ,( del_tx_id     , hash3           , 4       , 0          , 0      , 0  , 0      , 1024, NULL          , NULL             , TRUE          , 1024        )
              ,( ins_tx_id2    , hash5           , 6       , 0          , 0      , 0  , 0      , 1024, NULL          , NULL             , TRUE          , 1024        )
              ,( ups_tx_id     , hash7           , 7       , 2          , 0      , 0  , 0      , 1024, NULL          , NULL             , TRUE          , 1024        )
;

INSERT INTO tx_out ( id   , tx_id           , index, address     , address_raw, address_has_script, payment_cred, stake_address_id, value, data_hash )
            VALUES ( 0    , consumed_tx_id  , 0    , 'other_addr', ''         , TRUE              , NULL        , NULL            , 0    , NULL      )
                  ,( 1    , consumed_tx_id  , 1    , 'other_addr', ''         , TRUE              , NULL        , NULL            , 0    , NULL      )
                  ,( 2    , consumed_tx_id  , 2    , 'other_addr', ''         , TRUE              , NULL        , NULL            , 0    , NULL      )
                  ,( 4    , ins_tx_id       , 0    , script_addr , ''         , TRUE              , NULL        , NULL            , 0    , hash1     ) -- add key1
                  ,( 5    , ins_tx_id       , 1    , script_addr , ''         , TRUE              , NULL        , NULL            , 0    , hash2     ) -- add invalid
                  ,( 6    , ins_tx_ida      , 2    , script_addr , ''         , TRUE              , NULL        , NULL            , 0    , hash3     ) -- add key2a
                  ,( 7    , ins_tx_id       , 3    , script_addr , ''         , TRUE              , NULL        , NULL            , 0    , hash4     ) -- add key2 simulating 2 utxos with the same key
                  ,( 8    , del_tx_id       , 0    , 'other_addr', ''         , TRUE              , NULL        , NULL            , 0    , NULL      ) -- delete key1
                  ,( 9    , ins_tx_id2      , 0    , script_addr , ''         , TRUE              , NULL        , NULL            , 0    , hash5     ) -- add key3
                  ,( 10   , ups_tx_id       , 0    , script_addr , ''         , TRUE              , NULL        , NULL            , 0    , hash6     ) -- upsert key3
;

INSERT INTO ma_tx_out ( id   , quantity , tx_out_id , ident)
VALUES                ( 0    , 1        , 4         , 999  )
                     ,( 1    , 1        , 5         , 999  )
                     ,( 2    , 1        , 6         , 999  )
                     ,( 3    , 1        , 7         , 999  )
                     ,( 4    , 1        , 8         , 999  )
                     ,( 5    , 1        , 9         , 999  )
                     ,( 6    , 1        , 10        , 999  )
;

INSERT INTO tx_in ( id, tx_in_id   , tx_out_id     , tx_out_index, redeemer_id )
           VALUES ( 0 , ins_tx_id  , consumed_tx_id, 0           , NULL        )
                 ,( 1 , ins_tx_id  , consumed_tx_id, 1           , NULL        )
                 ,( 2 , ins_tx_id2 , consumed_tx_id, 2           , NULL        )
                 ,( 4 , del_tx_id  , ins_tx_id     , 0           , NULL        )
                 ,( 5 , del_tx_id  , ins_tx_id     , 1           , NULL        )
                 ,( 6 , ups_tx_id  , ins_tx_id2    , 0           , NULL        )
;

INSERT INTO datum ( id   , hash  , tx_id     , value                )
           VALUES ( 0    , hash1 , ins_tx_id , governed_map_entry_1 )
                 ,( 1    , hash2 , ins_tx_id , invalid_datum        )
                 ,( 2    , hash3 , ins_tx_ida, governed_map_entry_2a)
                 ,( 3    , hash4 , ins_tx_id , governed_map_entry_2 )
                 ,( 4    , hash5 , ins_tx_id2, governed_map_entry_3 )
                 ,( 6    , hash6 , ups_tx_id , governed_map_entry_3a)
;

END $$;
