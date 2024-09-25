pub struct DeregCmd;

impl CmdRun for DeregCmd {
	fn run<C: IOContext>(&self, context: &C) -> anyhow::Result<()> {
		let payment_vkey = hex!("a35ef86f1622172816bb9e916aea86903b2c8d32c728ad5c9b9472be7e3c5e88");
		let payment_key_hash = blake2b256(&payment_vkey);
		Ok(())
	}
}

/*
deregister ::
  forall r.
  DeregisterParams ->
  Run (APP + r) TransactionHash
deregister (DeregisterParams { sidechainParams, spoPubKey }) = do
  ownPkh <- getOwnPaymentPubKeyHash
  ownAddr <- getOwnWalletAddress
  validator <- getCommitteeCandidateValidator sidechainParams
  valAddr <- toAddress (PlutusScript.hash validator)
  ownUtxos <- Effect.utxosAt ownAddr
  valUtxos <- Effect.utxosAt valAddr

  { ownRegistrationUtxos } <- findOwnRegistrations ownPkh spoPubKey valUtxos

  when (null ownRegistrationUtxos)
	$ throw
		(NotFoundInputUtxo "Couldn't find registration UTxO")

  let
	lookups :: Lookups.ScriptLookups
	lookups = Lookups.validator validator
	  <> Lookups.unspentOutputs ownUtxos
	  <> Lookups.unspentOutputs valUtxos

	constraints :: Constraints.TxConstraints
	constraints = Constraints.mustBeSignedBy ownPkh
	  <> mconcat
		( flip Constraints.mustSpendScriptOutput (RedeemerDatum unit) <$>
			ownRegistrationUtxos
		)

  balanceSignAndSubmit "Deregister Committee Candidate" { lookups, constraints }

-- | Based on the wallet public key hash and the SPO public key, it finds the
-- | the registration UTxOs of the committee member/candidate
findOwnRegistrations ::
  forall r.
  PaymentPubKeyHash ->
  Maybe PubKey ->
  UtxoMap ->
  Run r
	{ ownRegistrationUtxos :: Array TransactionInput
	, ownRegistrationDatums :: Array BlockProducerRegistration
	}
findOwnRegistrations ownPkh spoPubKey validatorUtxos = do
  mayTxInsAndBlockProducerRegistrations <- Map.toUnfoldable validatorUtxos #
	traverse
	  \(input /\ TransactionOutput out) ->
		pure do
		  d <- outputDatumDatum =<< out.datum
		  BlockProducerRegistration r <- fromData d
		  guard
			( (getSPOPubKey r.stakeOwnership == spoPubKey) &&
				(r.ownPkh == ownPkh)
			)
		  pure (input /\ BlockProducerRegistration r)

  let
	txInsAndBlockProducerRegistrations = catMaybes
	  mayTxInsAndBlockProducerRegistrations
	ownRegistrationUtxos = map fst txInsAndBlockProducerRegistrations
	ownRegistrationDatums = map snd txInsAndBlockProducerRegistrations
  pure $ { ownRegistrationUtxos, ownRegistrationDatums }

*/
