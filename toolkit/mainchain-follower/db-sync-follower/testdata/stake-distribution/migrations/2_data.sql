do $$
declare

    pool_hash_raw_1 hash28type := decode('d5cfc42cf67f6b637688d19fa50a4342658f63370b9e2c9e3eaf4dfe', 'hex');
    pool_hash_raw_2 hash28type := decode('38f4a58aaf3fec84f3410520c70ad75321fb651ada7ca026373ce486', 'hex');
    pool_hash_raw_3 hash28type := decode('40d806d73c8d2a0c8d9b1e95ccb9f380e40cb4d4b23ff6e403ae1456', 'hex');

    pool_hash_view_1 text := 'pool16h8ugt8k0a4kxa5g6x062zjrgfjc7cehpw0ze8374axlul76932';
    pool_hash_view_2 text := 'pool18r62tz408lkgfu6pq5svwzkh2vslkeg6mf72qf3h8njgvzhx9ce';
    pool_hash_view_3 text := 'pool1grvqd4eu354qervmr62uew0nsrjqedx5kglldeqr4c29vv59rku';

    script_hash_1 hash28type := decode('49b16fb356be9e46778478f2c9601a24fa16c88b2a97681d5af06d01', 'hex');

    stake_hash_raw_0 addr29type := decode('e0ba149e2e2379097e65f0c03f2733d3103151e7f100d36dfdb01a0b22', 'hex');
    stake_hash_raw_1 addr29type := decode('f049b16fb356be9e46778478f2c9601a24fa16c88b2a97681d5af06d01', 'hex');
    stake_hash_raw_2 addr29type := decode('e0ad148225d7fb809f74a07d2dbc2eef91617f603bfb731e634bf8a1a9', 'hex');
    stake_hash_raw_3 addr29type := decode('e1aa898fce3be344c6be2d86fe1c5918675c9b0672cda8ab809d262824', 'hex');
    stake_hash_raw_4 addr29type := decode('f133916328baa83c42dbdcde825122ccf024ca3599c19ca6fb1697dc93', 'hex');
    stake_hash_raw_5 addr29type := decode('e1c55157ae1b08643719584c4972132ed210c64b02da80004cbd9b8c7f', 'hex');

    stake_view_0 text := 'stake_test1uzapf83wydusjln97rqr7fen6vgrz5087yqdxm0akqdqkgstjz8g4';
    stake_view_1 text := 'stake_test17pymzman26lfu3nhs3u09jtqrgj059kg3v4fw6qattcx6qgt82eah';
    stake_view_2 text := 'stake_test1uzk3fq396lacp8m55p7jm0pwa7gkzlmq80ahx8nrf0u2r2gefsccu';
    stake_view_3 text := 'stake_test1uz4gnr7w8035f3479kr0u8zerpn4excxwtx632uqn5nzsfq7jnzwv';
    stake_view_4 text := 'stake_test1uqeezcegh25rcskmmn0gy5fzenczfj34n8qeefhmz6taeycqg9wts';
    stake_view_5 text := 'stake_test1urz4z4awrvyxgdcetpxyjusn9mfpp3jtqtdgqqzvhkdcclcfrh0h4';

begin

INSERT INTO pool_hash
  (id, hash_raw       , "view"          )
VALUES
  (1 , pool_hash_raw_1, pool_hash_view_1),
  (2 , pool_hash_raw_2, pool_hash_view_2),
  (3 , pool_hash_raw_3, pool_hash_view_3)
;

INSERT INTO stake_address
  (id, hash_raw         , view         , script_hash   )
VALUES
  (0 , stake_hash_raw_0 , stake_view_0 , NULL          ),
  (1 , stake_hash_raw_1 , stake_view_1 , script_hash_1 ),
  (2 , stake_hash_raw_2 , stake_view_2 , NULL          ),
  (3 , stake_hash_raw_3 , stake_view_3 , NULL          ),
  (4 , stake_hash_raw_4 , stake_view_4 , script_hash_1 ),
  (5 , stake_hash_raw_5 , stake_view_5 , NULL          )
;


INSERT INTO epoch_stake
  (addr_id, pool_id, amount       , epoch_no)
VALUES
  (0      , 1      , 997652982    , 188     ),
  (1      , 1      , 1000000000000, 188     ),
  (2      , 1      , 0            , 188     ),
  (3      , 2      , 997825743    , 188     ),
  (4      , 2      , 5000000000000, 188     ),
  (5      , 2      , 997825743    , 188     ),
--(addr_id, pool_id, amount       , epoch_no)
  (0      , 1      , 997652982    , 189     ),
  (1      , 1      , 1000000000000, 189     ),
  (2      , 1      , 997825743    , 189     ),
  (3      , 2      , 997825743    , 189     ),
--(addr_id, pool_id, amount       , epoch_no)
  (4      , 1      , 871970938    , 190     ),
  (5      , 1      , 1000000000000, 190     ),
--(addr_id, pool_id, amount       , epoch_no)
  (0      , 1      , 997652982    , 193     ),
  (1      , 1      , 1000000000000, 193     ),
  (2      , 1      , 997825743    , 193     ),
  (3      , 2      , 997825743    , 193     ),
  (4      , 2      , 5000000000000, 193     ),
  (5      , 2      , 997825743    , 193     )
;

end $$;
