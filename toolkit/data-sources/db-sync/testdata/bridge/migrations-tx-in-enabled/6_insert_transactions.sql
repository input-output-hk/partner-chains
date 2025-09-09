DO $$
DECLARE
  reserve_datum jsonb := '{
					"list": [
						{ "constructor": 0, "fields": [] },
						{ "constructor": 1, "fields": [] },
						{ "int": 1 }
					]
		}';

  transfer_datum_1 jsonb := '{
					"list": [
						{ "constructor": 0, "fields": [] },
						{ "constructor": 0, "fields": [{ "bytes": "abcd" }] },
						{ "int": 1 }
					]
		}';

  transfer_datum_2 jsonb := '{
					"list": [
						{ "constructor": 0, "fields": [] },
						{ "constructor": 0, "fields": [{ "bytes": "1234" }] },
						{ "int": 1 }
					]
		}';

 invalid_datum jsonb := '{
        "list": [ { "int": 42 } ]
    }';


 native_token_policy hash28type := decode('500000000000000000000000000000000000434845434b504f494e69', 'hex');
 native_token_id integer := 1;
 irrelevant_token_id integer := 2;

 init_ics_tx integer = 1;
 reserve_transfer_tx integer   := 10;
 user_transfer_tx_1 integer := 21;
 user_transfer_tx_2 integer  := 22;
 invalid_transfer_tx_1 integer  := 31;
 irrelevant_tx integer  := 41;

 -- those hashes are not really important but putting them in variables help to make the data more readable
 init_ics_tx_hash hash32type := decode('c000000000000000000000000000000000000000000000000000000000000001','hex');
 reserve_transfer_tx_hash hash32type := decode('c000000000000000000000000000000000000000000000000000000000000002','hex');
 user_transfer_tx_hash_1 hash32type := decode('c000000000000000000000000000000000000000000000000000000000000003','hex');
 user_transfer_tx_hash_2 hash32type := decode('c000000000000000000000000000000000000000000000000000000000000004','hex');
 ivalid_transfer_tx_hash_1 hash32type := decode('c000000000000000000000000000000000000000000000000000000000000005','hex');
 irrelevant_tx_hash hash32type := decode('4242424242424242424242424242424242424242424242424242424242424242','hex');

 reserve_transfer_datum_hash hash32type := decode('0000000000000000000000000000000000000000000000000000000000000001','hex');
 user_tranfer_datum_hash_1 hash32type := decode('1000000000000000000000000000000000000000000000000000000000000001','hex');
 user_tranfer_datum_hash_2 hash32type := decode('1000000000000000000000000000000000000000000000000000000000000002','hex');
 invalid_transfer_datum hash32type := decode('1000000000000000000000000000000000000000000000000000000000000003','hex');

BEGIN

INSERT INTO tx ( id                    , hash                       , block_id, block_index, out_sum, fee, deposit, size, invalid_before, invalid_hereafter, valid_contract, script_size )
    VALUES     ( init_ics_tx           , init_ics_tx_hash           , 1       , 0          , 0      , 0  , 0      , 1024, NULL          , NULL             , TRUE          , 1024        )
              ,( irrelevant_tx         , irrelevant_tx_hash         , 1       , 1          , 0      , 0  , 0      , 1024, NULL          , NULL             , TRUE          , 1024        )
              ,( reserve_transfer_tx   , reserve_transfer_tx_hash   , 2       , 0          , 0      , 0  , 0      , 1024, NULL          , NULL             , TRUE          , 1024        )
              ,( user_transfer_tx_1    , user_transfer_tx_hash_1    , 2       , 1          , 0      , 0  , 0      , 1024, NULL          , NULL             , TRUE          , 1024        )
              ,( user_transfer_tx_2    , user_transfer_tx_hash_2    , 4       , 0          , 0      , 0  , 0      , 1024, NULL          , NULL             , TRUE          , 1024        )
              ,( invalid_transfer_tx_1 , ivalid_transfer_tx_hash_1  , 4       , 1          , 0      , 0  , 0      , 1024, NULL          , NULL             , TRUE          , 1024        )
;


INSERT INTO tx_out ( id, tx_id                 , index, address     , address_raw, address_has_script, payment_cred, stake_address_id, value, data_hash                    )
            VALUES ( 11, init_ics_tx           , 0    , 'ics address', ''         , TRUE              , NULL        , NULL            , 0    , NULL                        ) -- ICS initial utxo 1
                  ,( 12, init_ics_tx           , 1    , 'ics address', ''         , TRUE              , NULL        , NULL            , 0    , NULL                        ) -- ICS initial utxo 2
                  ,( 13, init_ics_tx           , 2    , 'ics address', ''         , TRUE              , NULL        , NULL            , 0    , NULL                        ) -- ICS initial utxo 3
                  ,( 14, init_ics_tx           , 3    , 'ics address', ''         , TRUE              , NULL        , NULL            , 0    , NULL                        ) -- ICS initial utxo 4
                  ,( 15, irrelevant_tx         , 0    , 'irrelevant' , ''         , TRUE              , NULL        , NULL            , 0    , NULL                        ) -- Irrelevant transaction with some native token
                  ,( 21, reserve_transfer_tx   , 0    , 'ics address', ''         , TRUE              , NULL        , NULL            , 0    , reserve_transfer_datum_hash ) -- transfers 100 tokens
                  ,( 31, user_transfer_tx_1    , 0    , 'ics address', ''         , TRUE              , NULL        , NULL            , 0    , user_tranfer_datum_hash_1   ) -- transfers 10 tokens + 100 tokens from previous transaction's utxo
                  ,( 32, user_transfer_tx_2    , 0    , 'ics address', ''         , TRUE              , NULL        , NULL            , 0    , user_tranfer_datum_hash_2   ) -- transfers 10 tokens + 110 tokens from previous transaction's utxo
                  ,( 41, invalid_transfer_tx_1 , 0    , 'ics address', ''         , TRUE              , NULL        , NULL            , 0    , invalid_transfer_datum      ) -- invalid transfer
;

INSERT INTO datum ( id, hash                        , tx_id                  , value            )
           VALUES ( 0 , reserve_transfer_datum_hash , reserve_transfer_tx    , reserve_datum    )
                 ,( 1 , user_tranfer_datum_hash_2   , user_transfer_tx_1     , transfer_datum_1 )
                 ,( 2 , user_tranfer_datum_hash_2   , user_transfer_tx_2     , transfer_datum_2 )
                 ,( 3 , invalid_transfer_datum      , invalid_transfer_tx_1  , invalid_datum    )
;

INSERT INTO multi_asset ( id                  , policy              , name               , fingerprint       )
VALUES                  ( native_token_id     , native_token_policy , 'native token'     , 'nativeToken'     )
                       ,( irrelevant_token_id , native_token_policy , 'irrelevant token' , 'irrelevantToken' )
;

INSERT INTO ma_tx_out (id , quantity , tx_out_id , ident )
VALUES                (11 , 100      , 21        , native_token_id )
                     ,(12 , 110      , 31        , native_token_id )
                     ,(13 , 120      , 32        , native_token_id )
                     ,(14 , 1000     , 41        , native_token_id )
                     ,(15 , 1000     , 15        , native_token_id )

                     ,(21 , 9999     , 21        , irrelevant_token_id )
                     ,(22 , 9999     , 31        , irrelevant_token_id )
                     ,(23 , 9999     , 32        , irrelevant_token_id )
                     ,(24 , 9999     , 41        , irrelevant_token_id )
                     ,(25 , 1000     , 15        , irrelevant_token_id )
;

INSERT INTO tx_in (id, tx_in_id               , tx_out_id           , tx_out_index, redeemer_id )
           VALUES (1 , reserve_transfer_tx    , init_ics_tx         , 0           , NULL        )
                 ,(3 , user_transfer_tx_1     , reserve_transfer_tx , 0           , NULL        )
                 ,(4 , user_transfer_tx_2     , user_transfer_tx_1  , 0           , NULL        )
                 ,(5 , invalid_transfer_tx_1  , user_transfer_tx_2  , 0           , NULL        )
                 ,(6 , user_transfer_tx_2     , irrelevant_tx       , 0           , NULL        )
;

END $$;
