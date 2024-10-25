-- This file insert various transactions into the test database
-- each section uses "namespaced" ids by using a range of thousands (ie., first section has ids 0xxx, second 1xxx etc)

DO $$
DECLARE
 reg_tx_id integer   := 0;
 dereg_tx_id integer := 1;
 consumed_tx_id integer  := 4;
 reg_tx_id2 integer  := 5;
 script_addr text    := 'script_addr';
 -- those hashes are not really important but putting them in variables help to make the data more readable
 hash1 hash32type := decode('ABEED7FB0067F14D6F6436C7F7DEDB27CE3CEB4D2D18FF249D43B22D86FAE3F1','hex');
 hash2 hash32type := decode('BBEED7FB0067F14D6F6436C7F7DEDB27CE3CEB4D2D18FF249D43B22D86FAE3F1','hex');
 hash3 hash32type := decode('CBEED7FB0067F14D6F6436C7F7DEDB27CE3CEB4D2D18FF249D43B22D86FAE3F1','hex');
 hash4 hash32type := decode('DBEED7FB0067F14D6F6436C7F7DEDB27CE3CEB4D2D18FF249D43B22D86FAE3F1','hex');
 hash5 hash32type := decode('055557FB0067F14D6F6436C7F7DEDB27CE3CEB4D2D18FF249D43B22D86FAE3F1','hex');
 hash6 hash32type := decode('01EED7FB0067F14D6F6436C7F7DEDB27CE3CEB4D2D18FF249D43B22D86FAE3F1','hex');
 consumed_tx_hash  hash32type := decode('cdefe62b0a0016c2ccf8124d7dda71f6865283667850cc7b471f761d2bc1eb13','hex');

 registration_spo_A jsonb := '{
    "constructor": 0,
    "fields": [
      {
      	"constructor": 0,
        "fields": [
        	{ "bytes": "bfbee74ab533f40979101057f96de62e95233f2a5216eb16b54106f09fd7350d" },
        	{ "bytes": "28d1c3b7df297a60d24a3f88bc53d7029a8af35e8dd876764fd9e7a24203a3482a98263cc8ba2ddc7dc8e7faea31c2e7bad1f00e28c43bc863503e3172dc6b0a" }
        ]
      },
      { "bytes": "02fe8d1eb1bcb3432b1db5833ff5f2226d9cb5e65cee430558c18ed3a3c86ce1af" },
      { "bytes": "f8ec6c7f935d387aaa1693b3bf338cbb8f53013da8a5a234f9c488bacac01af259297e69aee0df27f553c0a1164df827d016125c16af93c99be2c19f36d2f66e" },
      {
        "fields": [
          {
            "constructor": 0,
            "fields": [ { "bytes": "cdefe62b0a0016c2ccf8124d7dda71f6865283667850cc7b471f761d2bc1eb13"} ]
          },
          { "int": 1 }
        ],
        "constructor": 0
      },
      { "bytes": "aabbccddeeff00aabbccddeeff00aabbccddeeff00aabbccddeeff00" },
      { "bytes": "d43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d" },
      { "bytes": "88dc3417d5058ec4b4503e0c12ea1a0a89be200fe98922423d4334014fa6b0ee" }
    ]
  }';
 registration_SPO_B jsonb := '{
    "constructor": 0,
    "fields": [
      {
        "constructor": 0,
        "fields": [
          { "bytes": "cfbee74ab533f40979101057f96de62e95233f2a5216eb16b54106f09fd7350d" },
          { "bytes": "28d1c3b7df297a60d24a3f88bc53d7029a8af35e8dd876764fd9e7a24203a3482a98263cc8ba2ddc7dc8e7faea31c2e7bad1f00e28c43bc863503e3172dc6b0a" }
        ]
      },
      { "bytes": "02fe8d1eb1bcb3432b1db5833ff5f2226d9cb5e65cee430558c18ed3a3c86ce1af" },
      { "bytes": "f8ec6c7f935d387aaa1693b3bf338cbb8f53013da8a5a234f9c488bacac01af259297e69aee0df27f553c0a1164df827d016125c16af93c99be2c19f36d2f66e" },
      {
        "fields": [
          {
            "constructor": 0,
            "fields": [ { "bytes": "cdefe62b0a0016c2ccf8124d7dda71f6865283667850cc7b471f761d2bc1eb13"} ]
          },
          { "int": 1 }
        ],
        "constructor": 0
      },
      { "bytes": "aa112233445566aa112233445566aa112233445566aa112233445566" },
      { "bytes": "8eaf04151687736326c9fea17e25fc5287613693c912909cb226aa4794f26a48" },
      { "bytes": "d17c2d7823ebf260fd138f2d7e27d114c0145d968b5ff5006125f2414fadae69" }
    ]
  }';
 registration_SPO_C jsonb := '{
    "constructor": 0,
    "fields": [
      {
        "constructor": 0,
        "fields": [
          { "bytes": "3fd6618bfcb8d964f44beba4280bd91c6e87ac5bca4aa1c8f1cde9e85352660b" },
          { "bytes": "1fd2f1e5ad14c829c7359474764701cd74ab9c433c29b0bbafaa6bcf22376e9d651391d08ae6f40b418d2abf827c4c1fcb007e779a2beba7894d68012942c708" }
        ]
      },
      { "bytes": "02333e47cab242fefe88d7da1caa713307290291897f100efb911672d317147f72" },
      { "bytes": "3e8a8b29e513a08d0a66e22422a1a85d1bf409987f30a8c6fcab85ba38a85d0d27793df7e7fb63ace12203b062feb7edb5e6664ac1810b94c38182acc6167425" },
      {
        "fields": [
          {
            "constructor": 0,
            "fields": [ { "bytes": "cdefe62b0a0016c2ccf8124d7dda71f6865283667850cc7b471f761d2bc1eb13"} ]
          },
          { "int": 2 }
        ],
        "constructor": 0
      },
      { "bytes": "00112233445566001122334455660011223344556600112233445566" },
      { "bytes": "8eaf04151687736326c9fea17e25fc5287613693c912909cb226aa4794f23333" },
      { "bytes": "d17c2d7823ebf260fd138f2d7e27d114c0145d968b5ff5006125f2414fad3333" }
    ]
  }';
  -- this registration is invalid because of the consumed utxo which does not exist
 registration3 jsonb := '{
      "constructor": 0,
      "fields": [
      {
        "constructor": 0,
        "fields": [
          { "bytes": "3fd6618bfcb8d964f44beba4280bd91c6e87ac5bca4aa1c8f1cde9e85352660b" },
          { "bytes": "1fd2f1e5ad14c829c7359474764701cd74ab9c433c29b0bbafaa6bcf22376e9d651391d08ae6f40b418d2abf827c4c1fcb007e779a2beba7894d68012942c708" }
        ]
        },
        { "bytes": "02333e47cab242fefe88d7da1caa713307290291897f100efb911672d317147f72" },
        { "bytes": "3e8a8b29e513a08d0a66e22422a1a85d1bf409987f30a8c6fcab85ba38a85d0d27793df7e7fb63ace12203b062feb7edb5e6664ac1810b94c38182acc6167425" },
        {
          "fields": [
            {
              "constructor": 0,
              "fields": [ { "bytes": "bfbee74ab533f40979101057f96de62e95233f2a5216eb16b54106f09fd7350f"} ]
            },
            { "int": 2 }
          ],
          "constructor": 0
        },
        { "bytes": "ffffffffffeeeeaaae88d7da1caa713307290291897f100efb911672d317147f73" }
      ]
    }';
BEGIN

-- A first transaction is made to create some UTXO to consume
-- Then SPO A and B register during transaction identified by reg_tx_id (block id=0, epoch=189), it creates 2 registrations (second one overrides the first)
-- SPO A deregisters at block id 2 (epoch 190)
-- SPO C registers during block id 2 (epoch 191)
INSERT INTO tx ( id            , hash            , block_id, block_index, out_sum, fee, deposit, size, invalid_before, invalid_hereafter, valid_contract, script_size )

    VALUES     ( consumed_tx_id, consumed_tx_hash, 1       , 0          , 0      , 0  , 0      , 1024, NULL          , NULL             , TRUE          , 1024        )
              ,( reg_tx_id     , hash1           , 1       , 1          , 0      , 0  , 0      , 1024, NULL          , NULL             , TRUE          , 1024        )
              ,( dereg_tx_id   , hash2           , 4       , 0          , 0      , 0  , 0      , 1024, NULL          , NULL             , TRUE          , 1024        )
              ,( reg_tx_id2    , hash5           , 6       , 0          , 0      , 0  , 0      , 1024, NULL          , NULL             , TRUE          , 1024        )
;


-- the transaction reg_tx_id has 2 outputs
-- the second output (index 1) is consumed by dereg_tx_id
INSERT INTO tx_out ( id, tx_id           , index, address     , address_raw, address_has_script, payment_cred, stake_address_id, value, data_hash )
            VALUES ( 0 , consumed_tx_id  , 0    , 'other_addr', ''         , TRUE              , NULL        , NULL            , 0    , NULL      )
                  ,( 1 , consumed_tx_id  , 1    , 'other_addr', ''         , TRUE              , NULL        , NULL            , 0    , NULL      )
                  ,( 2 , consumed_tx_id  , 2    , 'other_addr', ''         , TRUE              , NULL        , NULL            , 0    , NULL      )
                  ,( 4 , reg_tx_id       , 0    , script_addr , ''         , TRUE              , NULL        , NULL            , 0    , hash1     ) -- good registration
                  ,( 5 , reg_tx_id       , 1    , script_addr , ''         , TRUE              , NULL        , NULL            , 0    , hash2     ) -- wrong format
                  ,( 6 , reg_tx_id       , 2    , script_addr , ''         , TRUE              , NULL        , NULL            , 0    , hash3     ) -- formatted properly but not an spo
                  ,( 7 , dereg_tx_id     , 0    , 'other_addr', ''         , TRUE              , NULL        , NULL            , 0    , hash1     )
                  ,( 8 , reg_tx_id2      , 0    , script_addr , ''         , TRUE              , NULL        , NULL            , 0    , hash5     )
                  ,( 9 , reg_tx_id2      , 1    , script_addr , ''         , TRUE              , NULL        , NULL            , 0    , hash6     ) -- formatted properly but did not consumed advertisedhash
;

INSERT INTO datum ( id, hash , tx_id     , value                                            )
           VALUES ( 0 , hash1, reg_tx_id , registration_spo_A                               )
                 ,( 1 , hash2, reg_tx_id , '{ "constructor": 0, "fields": [{ "int": 1 }] }' ) -- this transaction has the wrong payload
                 ,( 2 , hash3, reg_tx_id , registration_SPO_B                               )
                 ,( 4 , hash5, reg_tx_id2, registration_SPO_C                               )
                 ,( 5 , hash6, reg_tx_id2, registration3                                    )
;

INSERT INTO tx_in ( id, tx_in_id   , tx_out_id, tx_out_index, redeemer_id )
           VALUES ( 0 , reg_tx_id  , consumed_tx_id, 0      , NULL   )
                 ,( 1 , reg_tx_id  , consumed_tx_id, 1      , NULL   )
                 ,( 2 , reg_tx_id2 , consumed_tx_id, 2      , NULL   )
                 ,( 4 , dereg_tx_id, reg_tx_id     , 0      , NULL   )
                 ,( 5 , dereg_tx_id, reg_tx_id     , 1      , NULL   )
;
END $$;

-- Following section inserts dummy transaction to test getting utxos
DO $$
DECLARE
 tx1id integer   := 1001;
 tx2id integer   := 1002;
 owner_addr text := 'get_utxo_test_address';
 -- those hashes are not really important but putting them in variables help to make the data more readable
 hash1 hash32type := decode('0000000000000000000000000000000000000000000000000000000000001001','hex');
 hash2 hash32type := decode('0000000000000000000000000000000000000000000000000000000000001002','hex');
BEGIN

-- some UTXOs are created during block id 0 (epoch 189)
-- one is consumed and one is created during block 2 (epoch 190)
INSERT INTO tx ( id         , hash , block_id, block_index, out_sum, fee, deposit, size, invalid_before, invalid_hereafter, valid_contract, script_size )
    VALUES     ( tx1id      , hash1, 1       , 3          , 0      , 0  , 0      , 1024, NULL          , NULL             , TRUE          , 1024        )
              ,( tx2id      , hash2, 4       , 2          , 0      , 0  , 0      , 1024, NULL          , NULL             , TRUE          , 1024        )
;


INSERT INTO tx_out ( id  , tx_id, index, address     , address_raw, address_has_script, payment_cred, stake_address_id, value, data_hash )
            VALUES ( 1000, tx1id, 0    , owner_addr      , ''         , TRUE       , NULL               , NULL            , 0    , NULL      )
                  ,( 1001, tx1id, 1    , owner_addr      , ''         , TRUE       , NULL               , NULL            , 0    , NULL      )
                  ,( 1002, tx1id, 2    , owner_addr      , ''         , TRUE       , NULL               , NULL            , 0    , hash1     )
                  ,( 1003, tx1id, 3    , owner_addr      , ''         , TRUE       , NULL               , NULL            , 0    , hash2     )
                  ,( 1004, tx2id, 0    , owner_addr      , ''         , TRUE       , NULL               , NULL            , 0    , hash2     )
;

INSERT INTO datum ( id, hash , tx_id    , value                                            )
           VALUES ( 1000 , hash1, tx1id, '{ "constructor": 0, "fields": [{ "int": 42 }] }')
                 ,( 1001 , hash2, tx1id, '{ "constructor": 0, "fields": [{ "int": 1  }] }')
;

INSERT INTO tx_in ( id   , tx_in_id, tx_out_id, tx_out_index, redeemer_id )
           VALUES ( 1000 , tx2id   , tx1id    , 0           , NULL        )
                 ,( 1001 , tx2id   , tx1id    , 1           , NULL        )
;
END $$;


-- Following section inserts a mainchain to sidechain transaction
DO $$
DECLARE
 sender_addr text   := 'sender';
 input_addr  text   := 'input_addr'; -- simulates reminder of input going to other UTxO
 policy hash28type := decode('52424aa2e3243dabef86f064cd3497bc176e1ca51d3d7de836db5571', 'hex');
 asset_name_encoded asset32type := decode('4655454C', 'hex'); -- "FUEL" string bytes

 updatable_policy hash28type := decode('450a4aa2e3243dabef86f064cd3497bc176e1ca51d3d7de836db450a', 'hex');
 updatable_asset_name_encoded asset32type := decode('555044415441424C450A', 'hex'); -- "UPDATABLE" string bytes
 other_policy hash28type := decode('20000000000000000000000000000000000000000000000000000123', 'hex');

 xc_tx_id1 integer   := 2001;
 xc_tx_id2 integer   := 2002;
 xc_tx_id3 integer   := 2003;
 tx_hash1 hash32type := decode('EBEED7FB0067F14D6F6436C7F7DEDB27CE3CEB4D2D18FF249D43B22D86FAE3F1','hex');
 tx_hash2 hash32type := decode('EBEED7FB0067F14D6F6436C7F7DEDB27CE3CEB4D2D18FF249D43B22D86FAE3F2','hex');
 tx_hash3 hash32type := decode('EBEED7FB0067F14D6F6436C7F7DEDB27CE3CEB4D2D18FF249D43B22D86FAE3F3','hex');
 datum_hash1 hash32type := decode('AEDA0790DBEED7FB0067F14D6F6436C7F7DEDB27CE3CEB4D2D18FF249D43B22D','hex');
 datum_hash2 hash32type := decode('AEDA0790DBEED7FB0067F14D6F6436C7F7DEDB27CE3CEB4D2D18FF249D43B22E','hex');
 datum_hash3 hash32type := decode('AEDA0790DBEED7FB0067F14D6F6436C7F7DEDB27CE3CEB4D2D18FF249D43B22F','hex');

 upd_xc_tx_id1 integer   := 2004;
 upd_xc_tx_id2 integer   := 2005;
 upd_xc_tx_id3 integer   := 2006;
 upd_tx_hash1 hash32type := decode('EBEED7FB0067F14D6F6436C7F7DEDB27CE3CEB4D2D18FF249D43B22D86FAE3F4','hex');
 upd_tx_hash2 hash32type := decode('EBEED7FB0067F14D6F6436C7F7DEDB27CE3CEB4D2D18FF249D43B22D86FAE3F5','hex');
 upd_tx_hash3 hash32type := decode('EBEED7FB0067F14D6F6436C7F7DEDB27CE3CEB4D2D18FF249D43B22D86FAE3F6','hex');
 upd_datum_hash1 hash32type := decode('AEDA0790DBEED7FB0067F14D6F6436C7F7DEDB27CE3CEB4D2D18FF249D43B23A','hex');
 upd_datum_hash2 hash32type := decode('AEDA0790DBEED7FB0067F14D6F6436C7F7DEDB27CE3CEB4D2D18FF249D43B23B','hex');
 upd_datum_hash3 hash32type := decode('AEDA0790DBEED7FB0067F14D6F6436C7F7DEDB27CE3CEB4D2D18FF249D43B23C','hex');

 -- Algorithm to obtain following values:
 -- val (prvKey, pubKey) = ECDSA.generateKeyPair(new SecureRandom(Array.empty))
 -- val address = Address.fromPublicKey(pubKey).bytes // first field
 -- val signature = prvKey.sign(kec256(address)).toBytes // second field
 legacy_redeemer jsonb := '{
    "constructor": 0,
    "fields": [
      { "bytes": "CC95F2A1011728FC8B861B3C9FEEFBB4E7449B98" }
    ]
  }';

 updatable_redeemer jsonb := '{
    "constructor": 1,
    "fields": [
      { "int": 42 },
      {"bytes": "CC95F2A1011728FC8B861B3C9FEEFBB4E7449B98"}
    ]
 }';

  invalid_xc_tx_1_id integer := 2201;
  invalid_xc_tx_1_hash hash32type := decode('BBBED7FB0067F14D6F6436C7F7DEDB27CE3CEB4D2D18FF249D43B22D86FAE3F1','hex');
  invalid_xc_tx_1_datum_hash hash32type := decode('BBBA0790DBEED7FB0067F14D6F6436C7F7DEDB27CE3CEB4D2D18FF249D43B22D','hex');
  invalid_xc_tx_1_redeemer jsonb := '{
    "constructor": 1,
    "fields": []
  }';


BEGIN

INSERT INTO tx ( id                 , hash                 , block_id , block_index , out_sum , fee , deposit , size , invalid_before , invalid_hereafter , valid_contract , script_size )
        VALUES ( xc_tx_id1          , tx_hash1             , 6        , 1           , 0       , 0   , 0       , 1024 , NULL           , NULL              , TRUE           , 1024        )
              ,( xc_tx_id2          , tx_hash2             , 6        , 2           , 0       , 0   , 0       , 1024 , NULL           , NULL              , TRUE           , 1024        )
              ,( xc_tx_id3          , tx_hash3             , 7        , 0           , 0       , 0   , 0       , 1024 , NULL           , NULL              , TRUE           , 1024        )
              ,( invalid_xc_tx_1_id , invalid_xc_tx_1_hash , 7        , 1           , 0       , 0   , 0       , 1024 , NULL           , NULL              , TRUE           , 1024        )
              ,( upd_xc_tx_id1      , upd_tx_hash1         , 6        , 3           , 0       , 0   , 0       , 1024 , NULL           , NULL              , TRUE           , 1024        )
              ,( upd_xc_tx_id2      , upd_tx_hash2         , 6        , 4           , 0       , 0   , 0       , 1024 , NULL           , NULL              , TRUE           , 1024        )
              ,( upd_xc_tx_id3      , upd_tx_hash3         , 7        , 2           , 0       , 0   , 0       , 1024 , NULL           , NULL              , TRUE           , 1024        )
;

INSERT INTO tx_out ( id   , tx_id                 , index , address     , address_raw , address_has_script , payment_cred , stake_address_id , value , data_hash                  )
            VALUES ( 2005 , xc_tx_id1          , 0     , sender_addr , ''          , TRUE               , NULL         , NULL             , 0     , datum_hash1                ) -- MC to SC script address
                  ,( 2006 , xc_tx_id1          , 1     , input_addr  , ''          , FALSE              , NULL         , NULL             , 0     , NULL                       ) -- remainder of input goes to sender
                  ,( 2007 , xc_tx_id2          , 0     , sender_addr , ''          , TRUE               , NULL         , NULL             , 0     , datum_hash2                ) -- MC to SC script address
                  ,( 2008 , xc_tx_id3          , 0     , sender_addr , ''          , TRUE               , NULL         , NULL             , 0     , datum_hash3                ) -- MC to SC script address
                  ,( 2009 , invalid_xc_tx_1_id , 0     , sender_addr , ''          , FALSE              , NULL         , NULL             , 0     , invalid_xc_tx_1_datum_hash )
                  ,( 2011 , upd_xc_tx_id1       , 0     , sender_addr , ''          , TRUE               , NULL         , NULL             , 0     , upd_datum_hash1            ) -- MC to SC script address
                  ,( 2012 , upd_xc_tx_id1       , 1     , input_addr  , ''          , FALSE              , NULL         , NULL             , 0     , NULL                       ) -- remainder of input goes to sender
                  ,( 2013 , upd_xc_tx_id2       , 0     , sender_addr , ''          , TRUE               , NULL         , NULL             , 0     , upd_datum_hash2            ) -- MC to SC script address
                  ,( 2014 , upd_xc_tx_id3       , 0     , sender_addr , ''          , TRUE               , NULL         , NULL             , 0     , upd_datum_hash3            ) -- MC to SC script address
;

INSERT INTO multi_asset ( id , policy                , name                         , fingerprint)
                 VALUES ( 2013 , policy                , asset_name_encoded           , 'assetFUEL')
                       ,( 2014 , updatable_policy      , updatable_asset_name_encoded , 'assetUPDATABLE')
;

INSERT INTO ma_tx_mint ( id    , quantity , tx_id              , ident )
                VALUES ( 2001  , -500     , xc_tx_id1          , 2013  )
                      ,( 2002  , -500     , xc_tx_id2          , 2013  )
                      ,( 2003  , -500     , xc_tx_id3          , 2013  )
                      ,( 2004  , -400     , invalid_xc_tx_1_id , 2013  )
                      ,( 2006  , -500     , upd_xc_tx_id1      , 2014  )
                      ,( 2007  , -500     , upd_xc_tx_id2      , 2014  )
                      ,( 2008  , -500     , upd_xc_tx_id3      , 2014  )
;

INSERT INTO redeemer_data (id    , hash                       , tx_id              , value                    )
                   VALUES ( 2004 , datum_hash1                , xc_tx_id1          , legacy_redeemer          )
                         ,( 2005 , datum_hash2                , xc_tx_id2          , legacy_redeemer          )
                         ,( 2006 , datum_hash3                , xc_tx_id3          , legacy_redeemer          )
                         ,( 2007 , invalid_xc_tx_1_datum_hash , invalid_xc_tx_1_id , invalid_xc_tx_1_redeemer )
                         ,( 2009 , upd_datum_hash1            , upd_xc_tx_id1      , updatable_redeemer       )
                         ,( 2010 , upd_datum_hash2            , upd_xc_tx_id2      , updatable_redeemer       )
                         ,( 2011 , upd_datum_hash3            , upd_xc_tx_id3      , updatable_redeemer       )
;

INSERT INTO redeemer ( id    , tx_id              , unit_mem , unit_steps , fee , purpose , index , script_hash      , redeemer_data_id )
              VALUES ( 2000  , xc_tx_id1          , 0        , 0          , 0   , 'mint'  , 0     , policy           , 2004             )
                    ,( 2001  , xc_tx_id2          , 0        , 0          , 0   , 'mint'  , 0     , policy           , 2005             )
                    ,( 2002  , xc_tx_id3          , 0        , 0          , 0   , 'mint'  , 0     , policy           , 2006             )
                    ,( 2003  , invalid_xc_tx_1_id , 0        , 0          , 0   , 'mint'  , 0     , policy           , 2007             )
                    -- the new transactions have several mint redeemer per transaction
                    ,( 2005  , upd_xc_tx_id1      , 0        , 0          , 0   , 'mint'  , 0     , other_policy     , 2009             )
                    ,( 2006  , upd_xc_tx_id1      , 0        , 0          , 0   , 'mint'  , 1     , updatable_policy , 2009             )
                    ,( 2007  , upd_xc_tx_id2      , 0        , 0          , 0   , 'mint'  , 0     , other_policy     , 2007             )
                    ,( 2008  , upd_xc_tx_id2      , 0        , 0          , 0   , 'mint'  , 1     , updatable_policy , 2010             )
                    ,( 2009  , upd_xc_tx_id3      , 0        , 0          , 0   , 'mint'  , 0     , other_policy     , 2011             )
                    ,( 2010  , upd_xc_tx_id3      , 0        , 0          , 0   , 'mint'  , 1     , updatable_policy , 2011             )
;

END $$;

-- Committee handovers  --
DO $$
DECLARE
 tx1id integer   := 3001;
 tx2id integer   := 3002;
 owner_addr text := 'committee_test_address';
 tx_hash1 hash32type := decode('000000010067F14D6F6436C7F7DEDB27CE3CEB4D2D18FF249D43B22D86FAE3F1','hex');
 tx_hash2 hash32type := decode('000000020067F14D6F6436C7F7DEDB27CE3CEB4D2D18FF249D43B22D86FAE3F2','hex');
 datum_hash1 hash32type := decode('3000000000000000000000000000000000000000000000000000000000001003','hex');
 datum_hash2 hash32type := decode('3000000000000000000000000000000000000000000000000000000000001004','hex');
 policy hash28type := decode('636f6d6d697474656586f064cd3497bc176e1ca51d3d7de836db5571', 'hex');
BEGIN

INSERT INTO tx ( id    , hash     , block_id , block_index , out_sum , fee , deposit , size , invalid_before , invalid_hereafter , valid_contract , script_size )
    VALUES     ( tx1id , tx_hash1 , 6        , 5           , 0       , 0   , 0       , 1024 , NULL           , NULL              , TRUE           , 1024        )
              ,( tx2id , tx_hash2 , 7        , 3           , 0       , 0   , 0       , 1024 , NULL           , NULL              , TRUE           , 1024        )
;
INSERT INTO tx_out ( id   , tx_id , index , address    , address_raw , address_has_script , payment_cred , stake_address_id , value , data_hash                  )
            VALUES ( 3005 , tx1id , 0     , owner_addr , ''          , TRUE               , NULL         , NULL             , 0     , datum_hash1                ) -- first committee (consumed)
                  ,( 3006 , tx2id , 0     , owner_addr , ''          , FALSE              , NULL         , NULL             , 0     , datum_hash2                ) -- second committee
;

INSERT INTO tx_in ( id   , tx_in_id , tx_out_id , tx_out_index , redeemer_id )
           VALUES ( 3001 , tx2id    , tx1id     , 0            , NULL        ) -- consume the first committee
;

INSERT INTO multi_asset ( id , policy      , name               , fingerprint     )
                 VALUES ( 15 , policy      , ''                 , 'assetCommittee')
;

INSERT INTO ma_tx_out (id   , quantity , tx_out_id , ident)
            VALUES    (3001 , 1        , 3005      ,  15)
                     ,(3002 , 1        , 3006      ,  15)
;


INSERT INTO datum ( id   , hash        , tx_id , value                                                                                                                          )
           VALUES ( 3001 , datum_hash1 , tx1id , '{"fields": [{"bytes": "ffff2cd23dcd12169df205f8bf659554441885fb39393a82a5e3b13601aa8cab"}, {"int": 1113}], "constructor": 0}' )
                 ,( 3002 , datum_hash2 , tx2id , '{"list": [{"bytes": "d5462cd23dcd12169df205f8bf659554441885fb39393a82a5e3b13601aa8cab"}, {"int": 1114}]}' )
;
END $$;

-- Distributed set --
DO $$
DECLARE
 tx1id integer   := 4001;
 tx2id integer   := 4002;
 owner_addr text := 'distributed_set_test_address';
 tx_hash1 hash32type := decode('4000000000000000000000000000000000000000000000000000000000000000','hex');
 tx_hash2 hash32type := decode('4000000000000000000000000000000000000000000000000000000000000001','hex');
 datum_hash1 hash32type := decode('4000000000000000000000000000000000000000000000000000000000000001','hex');
 datum_hash2 hash32type := decode('4000000000000000000000000000000000000000000000000000000000000002','hex');
 datum_hash3 hash32type := decode('4000000000000000000000000000000000000000000000000000000000000003','hex');
 datum_hash4 hash32type := decode('4000000000000000000000000000000000000000000000000000000000000004','hex');
 policy hash28type := decode('40000000000000000000000000000000000000000000000036db5571', 'hex');
BEGIN

-- the set is fist broken into two utxos, the second is consumed to get 3 elements

INSERT INTO tx ( id    , hash     , block_id , block_index , out_sum , fee , deposit , size , invalid_before , invalid_hereafter , valid_contract , script_size )
    VALUES     ( tx1id , tx_hash1 , 6        , 6           , 0       , 0   , 0       , 1024 , NULL           , NULL              , TRUE           , 1024        )
              ,( tx2id , tx_hash2 , 7        , 4           , 0       , 0   , 0       , 1024 , NULL           , NULL              , TRUE           , 1024        )
;
INSERT INTO tx_out ( id   , tx_id , index , address    , address_raw , address_has_script , payment_cred , stake_address_id , value , data_hash                  )
            VALUES ( 4000 , tx1id , 0     , owner_addr , ''          , TRUE               , NULL         , NULL             , 0     , datum_hash1                ) -- first element
                  ,( 4001 , tx1id , 1     , owner_addr , ''          , TRUE               , NULL         , NULL             , 0     , datum_hash2                ) -- second element (consumed)
                  ,( 4002 , tx2id , 0     , owner_addr , ''          , TRUE               , NULL         , NULL             , 0     , datum_hash3                ) -- second (unconsumed)
                  ,( 4003 , tx2id , 1     , owner_addr , ''          , TRUE               , NULL         , NULL             , 0     , datum_hash4                ) -- third element
;

INSERT INTO tx_in ( id   , tx_in_id , tx_out_id , tx_out_index , redeemer_id )
           VALUES ( 4000 , tx2id    , tx1id     , 1            , NULL        ) -- consume the second element
;


INSERT INTO multi_asset ( id   , policy      , name                                                                            , fingerprint )
                 VALUES ( 4000 , policy      , decode('0000000000000000000000000000000000000000000000000000000000000000','hex'), ''          )
                       ,( 4001 , policy      , decode('0000000000000000000000000000000000000000000000000000000111111111','hex'), ''          )
                       ,( 4002 , policy      , decode('0000000000000000000000000000000000000000000000000000000222222222','hex'), ''          )
;

INSERT INTO ma_tx_out (id   , quantity , tx_out_id , ident)
            VALUES    (4000 , 1        , 4000      ,  4000)
                     ,(4001 , 1        , 4001      ,  4001)
                     ,(4002 , 1        , 4002      ,  4001)
                     ,(4003 , 1        , 4003      ,  4002)
;


INSERT INTO datum ( id   , hash        , tx_id , value                                                                                                           )
           VALUES ( 4000 , datum_hash1 , tx1id , '{"bytes": "0000000000000000000000000000000000000000000000000000000111111111" }' )
                 ,( 4001 , datum_hash2 , tx2id , '{"bytes": "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff" }' )
                 ,( 4002 , datum_hash3 , tx2id , '{"bytes": "0000000000000000000000000000000000000000000000000000000222222222" }' )
                 ,( 4003 , datum_hash4 , tx2id , '{"bytes": "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff" }' )
;
END $$;


-- Insert one checkpoint NFT  --
DO $$
    DECLARE
        tx1_id integer   := 5001;
        owner_addr text := 'checkpoint_test_address';
        tx1_hash hash32type := decode('5000000000000000000000000000000000000000000000000000000000000001', 'hex');
        datum1_hash hash32type := decode('5000000000000000000000000000000000000000000000000000000000000001', 'hex');
        checkpoint_nft_policy hash28type := decode('500000000000000000000000000000000000434845434b504f494e54', 'hex');
    BEGIN
        INSERT INTO tx ( id     , hash     , block_id , block_index , out_sum , fee , deposit , size , invalid_before , invalid_hereafter , valid_contract , script_size )
        VALUES         ( tx1_id , tx1_hash , 8        , 2           , 0       , 0   , 0       , 1024 , NULL           , NULL              , TRUE           , 1024        )
        ;

        INSERT INTO tx_out ( id   , tx_id  , index , address    , address_raw , address_has_script , payment_cred , stake_address_id , value , data_hash                  )
        VALUES             ( 5000 , tx1_id , 0     , owner_addr , ''          , TRUE               , NULL         , NULL             , 0     , datum1_hash                )
        ;

        INSERT INTO multi_asset ( id   , policy                , name      , fingerprint     )
        VALUES                  ( 5000 , checkpoint_nft_policy , ''        , 'assetCheckpointNft')
        ;

        INSERT INTO ma_tx_out (id   , quantity , tx_out_id , ident)
        VALUES                (5000 , 1        , 5000      , 5000)
        ;

        INSERT INTO datum ( id   , hash        , tx_id , value                                                                                                                         )
        VALUES            ( 5000 , datum1_hash , tx1_id, '{"list": [{"bytes": "abcd2cd23dcd12169df205f8bf659554441885fb39393a82a5e3b13601aa8cab"}, {"int": 667}]}' )
        ;
END $$;

-- Insert D-parameter
DO $$
    DECLARE
        tx1_id integer   := 6001;
        owner_addr text := 'd_parameter_test_address';
        tx1_hash hash32type := decode('5000000000000000000000000000000000000000000000000000000000000009', 'hex');
        datum1_hash hash32type := decode('5000000000000000000000000000000000000000000000000000000000000009', 'hex');
        d_parameter_policy hash28type := decode('500000000000000000000000000000000000434845434b504f494e69', 'hex');
    BEGIN
        INSERT INTO tx ( id     , hash     , block_id , block_index , out_sum , fee , deposit , size , invalid_before , invalid_hereafter , valid_contract , script_size )
        VALUES         ( tx1_id , tx1_hash , 3        , 0           , 0       , 0   , 0       , 1024 , NULL           , NULL              , TRUE           , 1024        )
        ;

        INSERT INTO tx_out ( id   , tx_id  , index , address    , address_raw , address_has_script , payment_cred , stake_address_id , value , data_hash                  )
        VALUES             ( 6000 , tx1_id , 0     , owner_addr , ''          , TRUE               , NULL         , NULL             , 0     , datum1_hash                )
        ;

        INSERT INTO multi_asset ( id   , policy                , name      , fingerprint     )
        VALUES                  ( 6000 , d_parameter_policy , ''        , 'assetDParameter')
        ;

        INSERT INTO ma_tx_out (id   , quantity , tx_out_id , ident)
        VALUES                (6000 , 1        , 6000      , 6000)
        ;

        INSERT INTO datum ( id   , hash        , tx_id , value                                                                                                                         )
        VALUES            ( 6000 , datum1_hash , tx1_id, '{"list": [{"int": 1}, {"int": 2}]}' )
        ;
END $$;

DO $$
    DECLARE
        tx1_id integer   := 7001;
        owner_addr text := 'd_parameter_test_address';
        tx1_hash hash32type := decode('5000000000000000000000000000000000000000000000000000000000000019', 'hex');
        datum1_hash hash32type := decode('5000000000000000000000000000000000000000000000000000000000000005', 'hex');
        d_parameter_policy hash28type := decode('500000000000000000000000000000000000434845434b504f494e69', 'hex');
    BEGIN
        INSERT INTO tx ( id     , hash     , block_id , block_index , out_sum , fee , deposit , size , invalid_before , invalid_hereafter , valid_contract , script_size )
        VALUES         ( tx1_id , tx1_hash , 4        , 10           , 0       , 0   , 0       , 1024 , NULL           , NULL              , TRUE           , 1024        )
        ;

        INSERT INTO tx_out ( id   , tx_id  , index , address    , address_raw , address_has_script , payment_cred , stake_address_id , value , data_hash                  )
        VALUES             ( 7000 , tx1_id , 0     , owner_addr , ''          , TRUE               , NULL         , NULL             , 0     , datum1_hash                )
        ;

        INSERT INTO ma_tx_out (id   , quantity , tx_out_id , ident)
        VALUES                (7000 , 1        , 7000      , 6000)
        ;

        INSERT INTO datum ( id   , hash        , tx_id , value                                                                                                                         )
        VALUES            ( 7000 , datum1_hash , tx1_id, '{"list": [{"int": 1}, {"int": 3}]}' )
        ;
END $$;

-- Insert permissioned candidates
DO $$
    DECLARE
        tx1_id integer   := 8001;
        owner_addr text := 'permissioned_candidates_test_address';
        tx1_hash hash32type := decode('5000000000000000000000000000000000000000000000000000000000000029', 'hex');
        datum1_hash hash32type := decode('5000000000000000000000000000000000000000000000000000000000000129', 'hex');
        permissioned_candidates_policy hash28type := decode('500000000000000000000000000000000000434845434b504f494e19', 'hex');
    BEGIN
        INSERT INTO tx ( id     , hash     , block_id , block_index , out_sum , fee , deposit , size , invalid_before , invalid_hereafter , valid_contract , script_size )
        VALUES         ( tx1_id , tx1_hash , 3        , 1           , 0       , 0   , 0       , 1024 , NULL           , NULL              , TRUE           , 1024        )
        ;

        INSERT INTO tx_out ( id   , tx_id  , index , address    , address_raw , address_has_script , payment_cred , stake_address_id , value , data_hash                  )
        VALUES             ( 8000 , tx1_id , 0     , owner_addr , ''          , TRUE               , NULL         , NULL             , 0     , datum1_hash                )
        ;

        INSERT INTO multi_asset ( id   , policy                , name      , fingerprint     )
        VALUES                  ( 8000 , permissioned_candidates_policy , ''        , 'assetPermissionedCandidates')
        ;

        INSERT INTO ma_tx_out (id   , quantity , tx_out_id , ident)
        VALUES                (8000 , 1        , 8000      , 8000)
        ;

        INSERT INTO datum ( id   , hash        , tx_id , value                                                                                                                         )
        VALUES            ( 8000 , datum1_hash , tx1_id, '{"list": [{"list": [{"bytes": "bb11"}, {"bytes": "cc11"}, {"bytes": "dd11"}]}, {"list": [{"bytes": "bb22"}, {"bytes": "cc22"}, {"bytes": "dd22"}]}]}' )
        ;
END $$;

DO $$
    DECLARE
        tx1_id integer   := 9001;
        owner_addr text := 'permissioned_candidates_test_address';
        tx1_hash hash32type := decode('5000000000000000000000000000000000000000000000000000000000000039', 'hex');
        datum1_hash hash32type := decode('5000000000000000000000000000000000000000000000000000000000000045', 'hex');
    BEGIN
        INSERT INTO tx ( id     , hash     , block_id , block_index , out_sum , fee , deposit , size , invalid_before , invalid_hereafter , valid_contract , script_size )
        VALUES         ( tx1_id , tx1_hash , 4        , 11          , 0       , 0   , 0       , 1024 , NULL           , NULL              , TRUE           , 1024        )
        ;

        INSERT INTO tx_out ( id   , tx_id  , index , address    , address_raw , address_has_script , payment_cred , stake_address_id , value , data_hash                  )
        VALUES             ( 9000 , tx1_id , 0     , owner_addr , ''          , TRUE               , NULL         , NULL             , 0     , datum1_hash                )
        ;

        INSERT INTO ma_tx_out (id   , quantity , tx_out_id , ident)
        VALUES                (9000 , 1        , 9000      , 8000)
        ;

        INSERT INTO datum ( id   , hash        , tx_id , value                                                                                                                         )
        VALUES            ( 9000 , datum1_hash , tx1_id, '{"list": [{"list": [{"bytes": "cb6df9de1efca7a3998a8ead4e02159d5fa99c3e0d4fd6432667390bb4726854"}, {"bytes": "bf20afa1c1a72af3341fa7a447e3f9eada9f3d054a7408fb9e49ad4d6e6559ec"}, {"bytes": "9042a40b0b1baa9adcead024432a923eac706be5e1a89d7f2f2d58bfa8f3c26d"}]}, {"list": [{"bytes": "79c3b7fc0b7697b9414cb87adcb37317d1cab32818ae18c0e97ad76395d1fdcf"}, {"bytes": "56d1da82e56e4cb35b13de25f69a3e9db917f3e13d6f786321f4b0a9dc153b19"}, {"bytes": "7392f3ea668aa2be7997d82c07bcfbec3ee4a9a4e01e3216d92b8f0d0a086c32"}]}]}' )
        ;
END $$;
