-- the integration test assume a securityParameter of 50, epoch duration of 1000 slots and a coeff of 1
-- block id 0 is like genesis block it has epoch_no, block_no, slot_no null
-- block id 2 and 5 are like epoch boundary blocks they have epoch_no but slot_no and no block_no NULL
INSERT INTO block (id, hash                                                                            , epoch_no, slot_no , epoch_slot_no, block_no, previous_id, slot_leader_id, size, "time"                     , tx_count, proto_major, proto_minor, vrf_key, op_cert, op_cert_counter)
VALUES            (0 , decode('000000000067F14D6F6436C7F7DEDB27CE3CEB4D2D18FF249D43B22D86FAE3F1','hex'), NULL    , 189410, NULL         , NULL    , NULL    , 0             , 1024, '2022-04-20T16:28:00Z'     , 9       , 0          , 0          , ''     , NULL   , NULL           )
     			 ,(1 , decode('0BEED7FB0067F14D6F6436C7F7DEDB27CE3CEB4D2D18FF249D43B22D86FAE3F1','hex'), 189     , 189410, 410          , 0       , 0       , 0             , 1024, '2022-04-21T16:28:00Z'     , 1       , 0          , 0          , ''     , NULL   , NULL           )
	             ,(2 , decode('000000000167F14D6F6436C7F7DEDB27CE3CEB4D2D18FF249D43B22D86FAE3F1','hex'), 190     , NULL  , NULL         , NULL    , 1       , 0             , 1024, '2022-04-21T16:44:30Z'     , 0       , 0          , 0          , ''     , NULL   , NULL           )
     			 ,(3 , decode('ABEED7FB0067F14D6F6436C7F7DEDB27CE3CEB4D2D18FF249D43B22D86FAE3F1','hex'), 190     , 190400, 400          , 1       , 2       , 0             , 1024, '2022-04-21T16:44:30Z'     , 1       , 0          , 0          , ''     , NULL   , NULL           )
                 ,(4 , decode('BBEED7FB0067F14D6F6436C7F7DEDB27CE3CEB4D2D18FF249D43B22D86FAE3F1','hex'), 190     , 190500, 500          , 2       , 3       , 0             , 1024, '2022-04-21T16:46:10Z'     , 1       , 0          , 0          , ''     , NULL   , NULL           )
	             ,(5 , decode('000000000267F14D6F6436C7F7DEDB27CE3CEB4D2D18FF249D43B22D86FAE3F1','hex'), 191     , NULL  , NULL         , NULL    , 4       , 0             , 1024, '2022-04-21T17:02:50Z'     , 0       , 0          , 0          , ''     , NULL   , NULL           )
                 ,(6 , decode('CBEED7FB0067F14D6F6436C7F7DEDB27CE3CEB4D2D18FF249D43B22D86FAE3F1','hex'), 191     , 191500, 500          , 3       , 5       , 0             , 1024, '2022-04-21T17:02:50Z'     , 1       , 0          , 0          , ''     , NULL   , NULL           )
                 ,(7 , decode('DBEED7FB0067F14D6F6436C7F7DEDB27CE3CEB4D2D18FF249D43B22D86FAE3F1','hex'), 192     , 192500, 500          , 4       , 6       , 0             , 1024, '2022-04-21T17:19:30Z'     , 1       , 0          , 0          , ''     , NULL   , NULL           )
                 ,(8 , decode('EBEED7FB0067F14D6F6436C7F7DEDB27CE3CEB4D2D18FF249D43B22D86FAE3F1','hex'), 193     , 193500, 500          , 5       , 7       , 0             , 1024, '2022-04-21T17:36:10Z'     , 1       , 0          , 0          , ''     , NULL   , NULL           )
;
-- sometimes the block number can be null so we add this block just to handle that case
INSERT INTO block (id , hash                                                                            , epoch_no, slot_no, epoch_slot_no, block_no, previous_id, slot_leader_id, "size", "time"                   , tx_count, proto_major, proto_minor, vrf_key, op_cert, op_cert_counter) VALUES
	                (100, decode('76B343FB174CE057060B76D9DA6B474A5C1720814B0F34EAB4E2EFBA115F2308','hex'), NULL    , NULL   , NULL         , NULL    , NULL       , 1             , 0     , '2022-06-06 23:00:00.000', 4       , 0          , 0          , NULL   , NULL   , NULL           );