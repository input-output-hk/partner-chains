use hex_literal::hex;
use serde_json::json;

pub(crate) fn test_update_governance_tx() -> serde_json::Value {
	json!({
		"body": {
			"inputs": [
				{
					// payment utxo, arbitrary
					"transaction_id": "1bc6eeebd308616860384b9748801d586a93a7291faedb464e73e9f6355e392b",
					"index": 0
				},
				{
					// governance utxo, contains the datum {VersionOrcacle, 32} and 1 governance token
					// at address addr_test1wqrlc9gqxnyyzwyzgtvrf77famec87zme6zfxgq2sq4up8gccxfnc
					"transaction_id": "40db7e41a67c5c560aa3d4bce389cb2eecd7c5f88188dbe472eb95069d1357b3",
					"index": 0
				}
			],
			"outputs": [
				{
					// VersionOracleValidator
					"address": "addr_test1wqrlc9gqxnyyzwyzgtvrf77famec87zme6zfxgq2sq4up8gccxfnc",
					"amount": {
						"coin": "3318700",
						"multiasset": {
							// VersionOraclePolicy
							"c11dee532646a9b226aac75f77ea7ae5fba9270674327c882794701e": {
								// "Version oracle"
								"56657273696f6e206f7261636c65": "1"
							}
						}
					},
					"plutus_data": {
						"Data": "{\"list\":[{\"int\":32},{\"bytes\":\"c11dee532646a9b226aac75f77ea7ae5fba9270674327c882794701e\"}]}"
					},
					"script_ref": {
						"PlutusScript": "5901d30100003323322323232323322323232222323232532323355333573466e20cc8c8c88c008004c058894cd4004400c884cc018008c010004c04488004c04088008c01000400840304034403c4c02d24010350543500300d37586ae84008dd69aba1357440026eb0014c040894cd400440448c884c8cd40514cd4c00cc04cc030dd6198009a9803998009a980380411000a40004400290080a400429000180300119112999ab9a33710002900009807a490350543600133003001002301522253350011300f49103505437002215333573466e1d20000041002133005337020089001000980991299a8008806910a999ab9a3371e00a6eb800840404c0100048c8cc8848cc00400c008d55ce80098031aab9e00137540026016446666aae7c00480348cd4030d5d080118019aba2002498c02888cccd55cf8009006119a8059aba100230033574400493119319ab9c00100512200212200130062233335573e0024010466a00e6eb8d5d080118019aba20020031200123300122337000040029000180191299a800880211099a802801180200089100109109119800802001919180080091198019801001000a61239f9f581c84ba05c28879b299a8377e62128adc7a0e0df3ac438ff95efc7c8443ff01ff0001"
					}
				},
				{
					// change returned
					"address": "addr_test1vpmd59ajuvm34d723r8q2qzyz9ylq0x9pygqn7vun8qgpkgs7y5hw",
					"amount": {
						"coin": "9922275427",
						// minted governance token
						"multiasset": {
							"a646474b8f5431261506b6c273d307c7569a4eb6c96b42dd4a29520a": {
								"": "1"
							}
						}
					},
					"plutus_data": null,
					"script_ref": null
				}
			],
			"fee": "297747",
			"ttl": null,
			"certs": null,
			"withdrawals": null,
			"update": null,
			"auxiliary_data_hash": null,
			"validity_start_interval": null,
			"mint": [
				[
					// governance token
					"a646474b8f5431261506b6c273d307c7569a4eb6c96b42dd4a29520a",
					{
						"": "1"
					}
				]
			],
			"script_data_hash": "584e5a3ce181103e1cd93cee6d36d5e947bf66b1c7d11ba75dc75a4872793ec7",
			"collateral": [
				{
					"transaction_id": "1bc6eeebd308616860384b9748801d586a93a7291faedb464e73e9f6355e392b",
					"index": 0
				}
			],
			"required_signers": [
				"76da17b2e3371ab7ca88ce0500441149f03cc5091009f99c99c080d9"
			],
			"network_id": null,
			"collateral_return": {
				"address": "addr_test1vpmd59ajuvm34d723r8q2qzyz9ylq0x9pygqn7vun8qgpkgs7y5hw",
				"amount": {
					"coin": "9922499316",
					"multiasset": null
				},
				"plutus_data": null,
				"script_ref": null
			},
			"total_collateral": "446621",
			"reference_inputs": null,
			"voting_procedures": null,
			"voting_proposals": null,
			"donation": null,
			"current_treasury_value": null
		},
		"witness_set": {
			"vkeys": null,
			"native_scripts": null,
			"bootstraps": null,
			"plutus_scripts": [
				"59084501000033233223232323233223232323232323232323232332232322222253353232323235002223232323232323232533553355335325335323302d2253350011502722135002225333573466e3c00801c4c0b00044c01800c0094cd4cc0b0894cd400440b4884c8d400c88d400488894ccd4008401c854cd400884cc0e4894cd4004402c884c94cd400c84d4004894cd4cc0ac009204015333573466e1d200233301e00d0244890e56657273696f6e206f7261636c6500133503f009004100410041001300400132335038001039333333357480044a0644a0644646660526eb0010004894cd54cd4cccccc0a800888014801080108c0dc004801084c0dc004400c84ccc0b0008010894cd4cccccc0b400888020801c801c801c8c0e800484ccc0bc008c0e8cc0c0010004880244018400d40c8940c8940c80bc402084020c010004c070c8c8c8cc0bc894cd4004400c884cd40c4008c010004008c0a4c05cdd619803180111000a4000604e602c6eb0cc014c0048800520025300733004330082001002480004c07d2401194552524f522d56455253494f4e2d43555252454e43592d303100221533500110022213023491194552524f522d56455253494f4e2d43555252454e43592d303100213026001150243233302175ca040002660066a600c660066600e4002002900011000a401042a66a002202e4426a00444a66a0062666ae68cdc4800a400003603844203a202c202e202c2a66aa66a6602a014016202e202c264a66aa66a600a6052602a6eb0cc010d4c01ccc010c00488005200022001480104060405c54cd54cd4c94cd4c8cc0288004004d55cf0008a812910a99a8008a999ab9a3370e90011aab9d0031302800215027221502937546600860024400290011080c880b880c080b880ba9803007880b080b1aab9e001375401aa004604e44a66a002202a4426464a66aa666ae68cdd799801180091000a4000a66a64a66a602e6aae78004540988c8854cd400454ccd5cd19b8748008d55ce802098011816980d1bac33006301a35573c6ea8cc018c068d55cf1baa01548001200015029221502b302f225335001150292215333573466ebccc020c070d55cf1baa002480000144c0b00084c010004dd5198011a980400891000a4004426466008646601840020026aae78dd5000a4000660066a60120024400290010b080c8999ab9a3371266601464666046eb94088004cc008c00488005200200b48810e56657273696f6e206f7261636c6500480000640604c01800c40614c01c00d40144c8894ccd5cd19b88001480004c0712410350543600133003001002302622253350011301c49103505437002215333573466e1d200000410021330053370200890010009191980f1aab9d0013233004200100135573c0026ea80048c88c008004c09488cccd55cf8009013919a81318021aba100230033574400493111191981391299a8008a40004426a00444a666ae68cdc78010048980380089803001802181311299a8008a40004426a00444a666ae68cdc78010038800898030019bad0053300e375a6ae84004dd71aba1357440026eb0010dd7002080989808249035054350030172233335573e0024032466a0306ae84008c00cd5d100124c44666ae68cdc38010008020018910010910009111111999999aba40062323300735573a0026aae78004dd5003918029bab00723004375800e460066eb401c8c008dd70038079111999aab9f0032003233002357420086ae8801002c8848cc00400c008c040894cd40044044884cd4048c8c8c94ccd5cd19b87480000084cc8848cc00400c008c8c8c94ccd5cd19b87480000084cc8848cc00400c008c8c8c94ccd5cd19b87480000084dd71aba1001130104901035054310035573c0046aae74004dd51aba1001375a6ae84d5d100089806a481035054310035573c0046aae74004dd51aba10013232325333573466e1d200000213232333322221233330010050040030023232325333573466e1d2000002133221233001003002301035742002660224646464a666ae68cdc3a4000004264244600400660286ae8400454ccd5cd19b87480080084c8ccc888488ccc00401401000cdd69aba1002375a6ae84004dd69aba1357440026ae880044c0592401035054310035573c0046aae74004dd50009aba135744002260249201035054310035573c0046aae74004dd51aba100333301175ca0206ae84008c8c8c94ccd5cd19b87480000084488800c54ccd5cd19b87480080084c84888c004010dd71aba100115333573466e1d20040021321222300200435742002260249201035054310035573c0046aae74004dd51aba10013300e75c6ae84d5d10009aba2001357440022601a9201035054310035573c0046aae74004dd51aba13574400226014921035054310035573c0046aae74004dd5001180200091919192999ab9a3370e900000109909118010019bae357420022a666ae68cdc3a400400426424460020066eb8d5d080089803249035054310035573c0046aae74004dd5000911919192999ab9a3370e90010010a8058a999ab9a3370e90000010980618029aba1001130064901035054310035573c0046aae74004dd5000919319ab9c00100413300175ceb488c88c008dd58009806911999aab9f001200f23233500f33008300635573a002600a6aae78004c010d5d10019aba100200512001221233001003002212230020031122001300622533500110072213350080023004001300522533500110062213350070023004001300422533500110052213350060023004001300322533500110042213350050023004001122002122122330010040032323001001223300330020020014c12bd8799fd8799f5820071ce86f4b21214f35df5e7f2931a10b67f4a11360e56c1e2bcd7978980adca5ff01ff0001"
			],
			"plutus_data": null,
			"redeemers": [
				{
					"tag": "Spend",
					"index": "1",
					"data": "{\"int\":32}",
					"ex_units": {
						"mem": "111",
						"steps": "222"
					}
				},
				{
					"tag": "Mint",
					"index": "0",
					"data": "{\"constructor\":0,\"fields\":[]}",
					"ex_units": {
						"mem": "333",
						"steps": "555"
					}
				}
			]
		},
		"is_valid": true,
		"auxiliary_data": null
	}
	)
}

pub const MULTI_SIG_POLICY: &[u8] = &hex!("5901ae5901ab010000323322323232323322323232222323232532323355333573466e20cc8c8c88c008004c058894cd4004400c884cc018008c010004c04488004c04088008c01000400840304034403c4c02d2410350543500300d37586ae84008dd69aba1357440026eb0014c040894cd400440448c884c8cd40514cd4c00cc04cc030dd6198009a9803998009a980380411000a40004400290080a400429000180300119112999ab9a33710002900009807a490350543600133003001002301522253350011300f49103505437002215333573466e1d20000041002133005337020089001000980991299a8008806910a999ab9a3371e00a6eb800840404c0100048c8cc8848cc00400c008d55ce80098031aab9e00137540026016446666aae7c00480348cd4030d5d080118019aba2002498c02888cccd55cf8009006119a8059aba100230033574400493119319ab9c00100512200212200130062233335573e0024010466a00e6eb8d5d080118019aba20020031200123300122337000040029000180191299a800880211099a8028011802000891001091091198008020019191800800911980198010010009");
pub const VERSION_ORACLE_POLICY: &[u8] = &hex!("590c65590c6201000032323233223232323232323233223232323232323232323233223232323232323232232323232322225335323232323233353232325333573466e1d200000213322122233002005004375a6ae84004dd71aba1357440022a666ae68cdc3a40040042664424446600200a0086eb4d5d08009bae357426ae8800454ccd5cd19b87480100084c84888c00c010dd69aba1001130314901035054310035573c0046aae74004dd50039191919299a998082481174552524f522d56455253494f4e2d504f4c4943592d3037003303422533500110332213235003223500122225333500210072153500522350172233532335005233500425333573466e3c0080045400c40b880b88cd401080b894ccd5cd19b8f00200115003102e153350032153350022133500223350022335002233500223303000200120312335002203123303000200122203122233500420312225333573466e1c01800c54ccd5cd19b8700500213302c00400110331033102c153350012102c102c133044225335001100e22132533500321350012253353302c00201c153353302c333027010502048810e56657273696f6e206f7261636c6500480084cd41200d0010401040104004c010004c8cd4104004108c094014403084020c010004c080c07cc04cdd619801180091000a40002a66a660229201174552524f522d56455253494f4e2d504f4c4943592d3038005335330342253350011033221325333573466e24ccc050c078cc01cd4c0c800c880052002500d4890e56657273696f6e206f7261636c65004800040044cd40d40e0004c010004c0a4c04cdd619801180091000a4008203844203a266022921174552524f522d56455253494f4e2d504f4c4943592d3039003300e500800a101b101b5302c002502a50042232533533010491174552524f522d56455253494f4e2d504f4c4943592d303100300c302a302930123758660026a6058660026a605801244002900011000a40002a66a6601e9201174552524f522d56455253494f4e2d504f4c4943592d30320033005003002133010491174552524f522d56455253494f4e2d504f4c4943592d3033005004101a101a502a2253353300e491174552524f522d56455253494f4e2d504f4c4943592d30340033004002001153353300f491174552524f522d56455253494f4e2d504f4c4943592d3035003300c500600813300f4901174552524f522d56455253494f4e2d504f4c4943592d30360050031019101913300f33300a30143233502835302900122001480214009400d2210e56657273696f6e206f7261636c65004800888cc0c0894cd400440bc884c8d400c88894ccd40084014854cd4008854d401888d404888cd4c8cd40148cd401094ccd5cd19b8f00200115003102920292335004202925333573466e3c0080045400c40a454cd400c854cd400884cd40088cd40088cd40088cd40088cc0ac00800480b08cd400880b08cc0ac0080048880b0888cd401080b08894ccd5cd19b8700600315333573466e1c0140084cc09c01000440b840b8409c54cd40048409c409c4cc0fc894cd40044034884c94cd400c84d4004894cd4cc09c00806454cd4cc0b403406054cd4cc09cccc088045406d2210e56657273696f6e206f7261636c6500480084cd410c0bc0104010401040104004c010004c8cd40f00040f4c080018402c401884018c010004c068c8c068c040dd619a8149a981500091000a4008a006266a04a6a604c0064400290000a99aa99a9a981299a8121a981280111000a400444a66a0022a042442a66a0022a666ae68cdc3a4000008260480042a046442a04a4260426eb80045407c840044c0a92401164552524f522d4f5241434c452d504f4c4943592d313000301a003102a1302949010350543500302722533500110102215333573466ebc024008404c4c01000488c8c8c94cd4c94cd4c8cc0b4894cd400454088884d4008894ccd5cd19b8f002007130270011300600300253353302c225335001102b2213235003223500122225333500210072153350022133039225335001100b2213253350032135001225335330210024810054ccd5cd19b8748008ccc07003406d2210e56657273696f6e206f7261636c6500133503d009004100410041001300400132335036001037301a0021008210083004001300e3232323302f225335001100322133502f00230040010023012300d37586600c6004440029000180818061bac33005300122001480094c094cc010cc098800400920001302a491194552524f522d56455253494f4e2d43555252454e43592d30310022153350011002221302e491194552524f522d56455253494f4e2d43555252454e43592d3031002130210011501f3233301e75ca03a002660066a6048660066604a4002002900011000a401042a66a00220264426a00444a66a0062666ae68cdc4800a400002e03044203220246aae78004dd50012810111191981491299a8008a40004426a00444a666ae68cdc78010048980380089803001802181411299a8008a40004426a00444a666ae68cdc780100388008980300191299a80089812001110a99a8008801110981400311299a8008806899ab9c00200c30212233335573e0024042466a0406ae84008c00cd5d100124c44666ae68cdc380100080500491999999aba4001250142501423232333002375800800244a6646aa66a6666660020064400c400a400a46036002400a42603600220084266600c00600a44a66a666666008004440124010401040104603c00242666012004603c246600200a00444014200e4444446666666ae900188c8cc01cd55ce8009aab9e001375400e4600a6eac01c8c010dd6003918019bad00723002375c00e0542006a02a4446666aae7c00c800c8cc008d5d08021aba2004023250142501401f301e225335001101d22133501e300f0023004001301d225335001101c22133501d0023004001301c225335001101b22133501c0023004001233300e75ca01a00244666ae68cdc7801000802001891001091000980b91299a800880b11099a80b8011802000980b11299a800880a91099a80b18040011802000980a91299a800880a11099a80a8011802000980a11299a800880991099a80a1802801180200091919192999ab9a3370e90000010999109198008018011919192999ab9a3370e90000010999109198008018011919192999ab9a3370e900000109bae35742002260369201035054310035573c0046aae74004dd51aba1001375a6ae84d5d10008980c2481035054310035573c0046aae74004dd51aba10013005357426ae880044c055241035054310035573c0046aae74004dd500091919192999ab9a3370e9000001099191999911109199980080280200180118039aba100333300a75ca0126ae84008c8c8c94ccd5cd19b87480000084488800c54ccd5cd19b87480080084c84888c004010dd71aba100115333573466e1d20040021321222300200435742002260329201035054310035573c0046aae74004dd51aba10013300875c6ae84d5d10009aba200135744002260289201035054310035573c0046aae74004dd500091919192999ab9a3370e90000010991991091980080180118009aba10023300623232325333573466e1d200000213212230020033005357420022a666ae68cdc3a400400426466644424466600200a0080066eb4d5d08011bad357420026eb4d5d09aba200135744002260309201035054310035573c0046aae74004dd50009aba1357440044646464a666ae68cdc3a400000426424460040066eb8d5d08008a999ab9a3370e900100109909118008019bae357420022602e921035054310035573c0046aae74004dd500089809a49035054310035573c0046aae74004dd5000911919192999ab9a3370e90010010a8040a999ab9a3370e90000010980498029aba1001130134901035054310035573c0046aae74004dd5000899800bae75a4464460046eac004c04088cccd55cf800900811919a8081980798031aab9d001300535573c00260086ae8800cd5d0801008909118010018891000980591299a800880511099a8058011802000980511299a800880491099a8050011802000980491299a800880411099a80499a8029a980300111000a4000600800226444a666ae68cdc4000a4000260129210350543600133003001002300822253350011300949103505437002215333573466e1d20000041002133005337020089001000919198021aab9d0013233004200100135573c0026ea80048c88c008004c01c88cccd55cf8009003919a80318021aba10023003357440049311091980080180109100109109119800802001919319ab9c0010021200123230010012233003300200200101");
pub const VERSION_ORACLE_VALIDATOR: &[u8] = &hex!("5908195908160100003233223232323233223232323232323232323232332232322222253353232323235002223232323232323232533553355335325335323302d2253350011502722135002225333573466e3c00801c4c0b00044c01800c0094cd4cc0b0894cd400440b4884c8d400c88d400488894ccd4008401c854cd400884cc0e4894cd4004402c884c94cd400c84d4004894cd4cc0ac009204015333573466e1d200233301e00d02448810e56657273696f6e206f7261636c6500133503f009004100410041001300400132335038001039333333357480044a0644a0644646660526eb0010004894cd54cd4cccccc0a800888014801080108c0dc004801084c0dc004400c84ccc0b0008010894cd4cccccc0b400888020801c801c801c8c0e800484ccc0bc008c0e8cc0c0010004880244018400d40c8940c8940c80bc402084020c010004c070c8c8c8cc0bc894cd4004400c884cd40c4008c010004008c0a4c05cdd619803180111000a4000604e602c6eb0cc014c0048800520025300733004330082001002480004c07d2401194552524f522d56455253494f4e2d43555252454e43592d303100221533500110022213023491194552524f522d56455253494f4e2d43555252454e43592d303100213026001150243233302175ca040002660066a600c660066600e4002002900011000a401042a66a002202e4426a00444a66a0062666ae68cdc4800a400003603844203a202c202e202c2a66aa66a6602a014016202e202c264a66aa66a600a6052602a6eb0cc010d4c01ccc010c00488005200022001480104060405c54cd54cd4c94cd4c8cc0288004004d55cf0008a812910a99a8008a999ab9a3370e90011aab9d0031302800215027221502937546600860024400290011080c880b880c080b880ba9803007880b080b1aab9e001375401aa004604e44a66a002202a4426464a66aa666ae68cdd799801180091000a4000a66a64a66a602e6aae78004540988c8854cd400454ccd5cd19b8748008d55ce802098011816980d1bac33006301a35573c6ea8cc018c068d55cf1baa01548001200015029221502b302f225335001150292215333573466ebccc020c070d55cf1baa002480000144c0b00084c010004dd5198011a980400891000a4004426466008646601840020026aae78dd5000a4000660066a60120024400290010b080c8999ab9a3371266601464666046eb94088004cc008c00488005200200b48810e56657273696f6e206f7261636c6500480000640604c01800c40614c01c00d40144c8894ccd5cd19b88001480004c0712410350543600133003001002302622253350011301c49103505437002215333573466e1d200000410021330053370200890010009191980f1aab9d0013233004200100135573c0026ea80048c88c008004c09488cccd55cf8009013919a81318021aba100230033574400493111191981391299a8008a40004426a00444a666ae68cdc78010048980380089803001802181311299a8008a40004426a00444a666ae68cdc78010038800898030019bad0053300e375a6ae84004dd71aba1357440026eb0010dd7002080989808249035054350030172233335573e0024032466a0306ae84008c00cd5d100124c44666ae68cdc38010008020018910010910009111111999999aba40062323300735573a0026aae78004dd5003918029bab00723004375800e460066eb401c8c008dd70038079111999aab9f0032003233002357420086ae8801002c8848cc00400c008c040894cd40044044884cd4048c8c8c94ccd5cd19b87480000084cc8848cc00400c008c8c8c94ccd5cd19b87480000084cc8848cc00400c008c8c8c94ccd5cd19b87480000084dd71aba1001130104901035054310035573c0046aae74004dd51aba1001375a6ae84d5d100089806a481035054310035573c0046aae74004dd51aba10013232325333573466e1d200000213232333322221233330010050040030023232325333573466e1d2000002133221233001003002301035742002660224646464a666ae68cdc3a4000004264244600400660286ae8400454ccd5cd19b87480080084c8ccc888488ccc00401401000cdd69aba1002375a6ae84004dd69aba1357440026ae880044c0592401035054310035573c0046aae74004dd50009aba135744002260249201035054310035573c0046aae74004dd51aba100333301175ca0206ae84008c8c8c94ccd5cd19b87480000084488800c54ccd5cd19b87480080084c84888c004010dd71aba100115333573466e1d20040021321222300200435742002260249201035054310035573c0046aae74004dd51aba10013300e75c6ae84d5d10009aba2001357440022601a9201035054310035573c0046aae74004dd51aba13574400226014921035054310035573c0046aae74004dd5001180200091919192999ab9a3370e900000109909118010019bae357420022a666ae68cdc3a400400426424460020066eb8d5d080089803249035054310035573c0046aae74004dd5000911919192999ab9a3370e90010010a8058a999ab9a3370e90000010980618029aba1001130064901035054310035573c0046aae74004dd5000919319ab9c00100413300175ceb488c88c008dd58009806911999aab9f001200f23233500f33008300635573a002600a6aae78004c010d5d10019aba100200512001221233001003002212230020031122001300622533500110072213350080023004001300522533500110062213350070023004001300422533500110052213350060023004001300322533500110042213350050023004001122002122122330010040032323001001223300330020020011");