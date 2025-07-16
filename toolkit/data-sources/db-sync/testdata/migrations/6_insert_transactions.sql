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

-- uses legacy datum format
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
-- uses versioned v0 datum format
registration_SPO_B jsonb := '
{
  "list": [
    {"bytes": "aa112233445566aa112233445566aa112233445566aa112233445566"},
    {
      "constructor": 0,
      "fields": [
        {
          "constructor": 0,
          "fields": [
            {"bytes": "cfbee74ab533f40979101057f96de62e95233f2a5216eb16b54106f09fd7350d"},
            {"bytes": "28d1c3b7df297a60d24a3f88bc53d7029a8af35e8dd876764fd9e7a24203a3482a98263cc8ba2ddc7dc8e7faea31c2e7bad1f00e28c43bc863503e3172dc6b0a"}
          ]
        },
        {"bytes": "02fe8d1eb1bcb3432b1db5833ff5f2226d9cb5e65cee430558c18ed3a3c86ce1af"},
        {"bytes": "f8ec6c7f935d387aaa1693b3bf338cbb8f53013da8a5a234f9c488bacac01af259297e69aee0df27f553c0a1164df827d016125c16af93c99be2c19f36d2f66e"},
        {
          "constructor": 0,
          "fields": [
            {
              "constructor": 0,
              "fields": [{"bytes": "cdefe62b0a0016c2ccf8124d7dda71f6865283667850cc7b471f761d2bc1eb13"}]
            },
            {"int": 1}
          ]
        },
        {"bytes": "8eaf04151687736326c9fea17e25fc5287613693c912909cb226aa4794f26a48"},
        {"bytes": "d17c2d7823ebf260fd138f2d7e27d114c0145d968b5ff5006125f2414fadae69"}
      ]
    },
    {"int": 0}
  ]
}';
-- uses versioned v1 datum format
registration_SPO_C jsonb := '
{
	"list": [
	{"bytes": "00112233445566001122334455660011223344556600112233445566"},
    {
      "constructor": 0,
      "fields": [
        {
          "constructor": 0,
          "fields": [
            {"bytes": "3fd6618bfcb8d964f44beba4280bd91c6e87ac5bca4aa1c8f1cde9e85352660b"},
            {"bytes": "1fd2f1e5ad14c829c7359474764701cd74ab9c433c29b0bbafaa6bcf22376e9d651391d08ae6f40b418d2abf827c4c1fcb007e779a2beba7894d68012942c708"}
          ]
        },
        {"list": [{"bytes": "63726368"}, {"bytes": "02333e47cab242fefe88d7da1caa713307290291897f100efb911672d317147f72"}]},
        {"bytes": "3e8a8b29e513a08d0a66e22422a1a85d1bf409987f30a8c6fcab85ba38a85d0d27793df7e7fb63ace12203b062feb7edb5e6664ac1810b94c38182acc6167425"},
        {
          "constructor": 0,
          "fields": [
            {
              "constructor": 0,
              "fields": [{"bytes": "cdefe62b0a0016c2ccf8124d7dda71f6865283667850cc7b471f761d2bc1eb13"}]
            },
            {"int": 2}
          ]
        },
        {
          "list": [
            {"list": [{"bytes": "61757261"}, {"bytes": "8eaf04151687736326c9fea17e25fc5287613693c912909cb226aa4794f23333"}]},
            {"list": [{"bytes": "6772616e"}, {"bytes": "d17c2d7823ebf260fd138f2d7e27d114c0145d968b5ff5006125f2414fad3333"}]}
          ]
        }
      ]
    },
    {"int": 1}
  ]
}
';
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
INSERT INTO tx_out ( id, tx_id           , index, address     , address_raw, address_has_script, payment_cred, stake_address_id, value, data_hash, consumed_by_tx_id )
            VALUES ( 0 , consumed_tx_id  , 0    , 'other_addr', ''         , TRUE              , NULL        , NULL            , 0    , NULL     , reg_tx_id         )
                  ,( 1 , consumed_tx_id  , 1    , 'other_addr', ''         , TRUE              , NULL        , NULL            , 0    , NULL     , reg_tx_id         )
                  ,( 2 , consumed_tx_id  , 2    , 'other_addr', ''         , TRUE              , NULL        , NULL            , 0    , NULL     , reg_tx_id2        )
                  ,( 4 , reg_tx_id       , 0    , script_addr , ''         , TRUE              , NULL        , NULL            , 0    , hash1    , dereg_tx_id       ) -- good registration
                  ,( 5 , reg_tx_id       , 1    , script_addr , ''         , TRUE              , NULL        , NULL            , 0    , hash2    , dereg_tx_id       ) -- wrong format
                  ,( 6 , reg_tx_id       , 2    , script_addr , ''         , TRUE              , NULL        , NULL            , 0    , hash3    , NULL              ) -- formatted properly but not an spo
                  ,( 7 , dereg_tx_id     , 0    , 'other_addr', ''         , TRUE              , NULL        , NULL            , 0    , hash1    , NULL              )
                  ,( 8 , reg_tx_id2      , 0    , script_addr , ''         , TRUE              , NULL        , NULL            , 0    , hash5    , NULL              )
                  ,( 9 , reg_tx_id2      , 1    , script_addr , ''         , TRUE              , NULL        , NULL            , 0    , hash6    , NULL              ) -- formatted properly but did not consumed advertisedhash
;

INSERT INTO datum ( id, hash , tx_id     , value                                            )
           VALUES ( 0 , hash1, reg_tx_id , registration_spo_A                               )
                 ,( 1 , hash2, reg_tx_id , '{ "constructor": 0, "fields": [{ "int": 1 }] }' ) -- this transaction has the wrong payload
                 ,( 2 , hash3, reg_tx_id , registration_SPO_B                               )
                 ,( 4 , hash5, reg_tx_id2, registration_SPO_C                               )
                 ,( 5 , hash6, reg_tx_id2, registration3                                    )
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
        VALUES         ( tx1_id , tx1_hash , 1        , 2           , 0       , 0   , 0       , 1024 , NULL           , NULL              , TRUE           , 1024        )
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
        -- uses versioned v0 format of datum
        INSERT INTO datum ( id   , hash        , tx_id , value                                                                                                                         )
        VALUES            ( 8000 , datum1_hash , tx1_id, '{"list": [
        	{"constructor": 0, "fields": [] },
         	{"list": [
         		{"list": [{"bytes": "bb11"}, {"bytes": "cc11"}, {"bytes": "dd11"}]},
          		{"list": [{"bytes": "bb22"}, {"bytes": "cc22"}, {"bytes": "dd22"}]}
            ]},
            {"int": 0}
        ]}' )
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
        -- uses versioned v1 format
        INSERT INTO datum ( id   , hash        , tx_id , value                                                                                                                         )
        VALUES            ( 9000 , datum1_hash , tx1_id, '{"list": [
        	{"constructor": 0, "fields": [] },
         	{"list": [
            	{"list": [
             		{"list":[{"bytes": "63726368"}, {"bytes": "cb6df9de1efca7a3998a8ead4e02159d5fa99c3e0d4fd6432667390bb4726854"}]},
               		{"list":[
                 		{"list":[{"bytes": "61757261"}, {"bytes": "bf20afa1c1a72af3341fa7a447e3f9eada9f3d054a7408fb9e49ad4d6e6559ec"}]},
                   		{"list":[{"bytes": "6772616e"}, {"bytes": "9042a40b0b1baa9adcead024432a923eac706be5e1a89d7f2f2d58bfa8f3c26d"}]}
                    ]}
                ]},
                {"list": [
                	{"list":[{"bytes": "63726368"}, {"bytes": "79c3b7fc0b7697b9414cb87adcb37317d1cab32818ae18c0e97ad76395d1fdcf"}]},
                 	{"list":[
                 		{"list":[{"bytes": "61757261"}, {"bytes": "56d1da82e56e4cb35b13de25f69a3e9db917f3e13d6f786321f4b0a9dc153b19"}]},
                   		{"list":[{"bytes": "6772616e"}, {"bytes": "7392f3ea668aa2be7997d82c07bcfbec3ee4a9a4e01e3216d92b8f0d0a086c32"}]}
                    ]}
                ]}
            ]},
          	{"int": 1}
        ]}' )
        ;
END $$;
