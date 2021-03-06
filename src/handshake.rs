use crate::{
    credential::Credential,
    crypto::{ciphersuite::CipherSuite, dh::DhPoint, ecies::EciesCiphertext, sig::Signature},
    group_state::GroupState,
};

/// This contains the encrypted `WelcomeInfo` for new group participants
#[derive(Deserialize, Serialize)]
struct Welcome {
    // opaque user_init_key_id<0..255>;
    #[serde(rename = "user_init_key_id__bound_u8")]
    user_init_key_id: Vec<u8>,
    cipher_suite: &'static CipherSuite,
    encrypted_welcome_info: EciesCiphertext,
}

/// Contains a node's new public key and the new node's secret, encrypted for everyone in that
/// node's resolution
#[derive(Deserialize, Serialize)]
struct DirectPathNodeMessage {
    public_key: DhPoint,
    // ECIESCiphertext node_secrets<0..2^16-1>;
    node_secrets: Vec<EciesCiphertext>,
}

/// Contains a direct path of node messages. The length of `node_secrets` for the first
/// `DirectPathNodeMessage` MUST be zero.
#[derive(Deserialize, Serialize)]
struct DirectPathMessage {
    // DirectPathNodeMessage nodes<0..2^16-1>;
    node_messages: Vec<DirectPathNodeMessage>,
}

/// This is used in lieu of negotiating public keys when a participant is added. This has a bunch
/// of published ephemeral keys that can be used to initiated communication with a previously
/// uncontacted participant.
#[derive(Serialize)]
struct UserInitKey {
    // opaque user_init_key_id<0..255>
    /// An identifier for this init key. This MUST be unique among the `UserInitKey` generated by
    /// the client
    #[serde(rename = "user_init_key_id__bound_u8")]
    user_init_key_id: Vec<u8>,
    // CipherSuite cipher_suites<0..255>
    /// The cipher suites supported by this client. Each cipher suite here corresponds uniquely to
    /// a DH public key in `init_keys`. As such, this MUST have the same length as `init_keys`.
    #[serde(rename = "cipher_suites__bound_u8")]
    cipher_suites: Vec<&'static CipherSuite>,
    // DHPublicKey init_keys<1..2^16-1>
    /// The DH public keys owned by this client. Each public key corresponds uniquely to a cipher
    /// suite in `cipher_suites`. As such, this MUST have the same length as `cipher_suites`.
    #[serde(rename = "init_keys__bound_u16")]
    init_keys: Vec<DhPoint>,
    /// The identity information of this user
    credential: Credential,
    /// Contains the signature of all the other fields of this struct, under the identity key of
    /// the client.
    // opaque signature<0..2^16-1>
    signature: Signature,
}

/// This is currently not defined by the spec. See open issue in section 7.1
#[derive(Serialize)]
struct GroupInit;

/// Operation to add a partcipant to a group
#[derive(Serialize)]
struct GroupAdd {
    init_key: UserInitKey,
}

/// Operation to add entropy to the group
#[derive(Serialize)]
struct GroupUpdate {
    path: DirectPathMessage,
}

/// Operation to remove a partcipant from the group
#[derive(Serialize)]
struct GroupRemove {
    removed: u32,
    path: DirectPathMessage,
}

/// Enum of possible group operations
#[derive(Serialize)]
#[serde(rename = "GroupOperation__enum_u8")]
enum GroupOperation {
    Init(GroupInit),
    Add(GroupAdd),
    Update(GroupUpdate),
    Remove(GroupRemove),
}

struct Handshake {
    /// This is equal to the epoch of the current `GroupState`
    prior_epoch: u32,
    /// The operation this `Handshake` is perofrming
    operation: GroupOperation,
    /// Position of the signer in the roster
    signer_index: u32,
    /// Signature over the `Group`'s history:
    /// `Handshake.signature = Sign(identity_key, GroupState.transcript_hash)`
    signature: Signature,
    /// HMAC over the group state and `Handshake` signature
    /// `confirmation_data = GroupState.transcript_hash || Handshake.signature`
    /// `Handshake.confirmation = HMAC(confirmation_key, confirmation_data)`
    confirmation: ring::hmac::Signature,
}

impl Handshake {
    /// Creates a `Handshake` message, given a ciphersuite, group state, and group operation
    fn from_group_op(
        cs: &'static CipherSuite,
        state: &GroupState,
        op: GroupOperation,
    ) -> Handshake {
        // signature = Sign(identity_key, GroupState.transcript_hash)
        let signature = cs
            .sig_impl
            .sign(&state.identity_key, &state.transcript_hash);

        // confirmation_data = GroupState.transcript_hash || Handshake.signature
        let confirmation_data = [
            state.transcript_hash.as_slice(),
            cs.sig_impl.signature_to_bytes(&signature).as_slice(),
        ]
        .concat();
        // confirmation = HMAC(confirmation_key, confirmation_data)
        let confirmation = ring::hmac::sign(&state.confirmation_key, &confirmation_data);

        Handshake {
            prior_epoch: state.epoch,
            operation: op,
            signer_index: state.my_position_in_roster,
            signature: signature,
            confirmation: confirmation,
        }
    }
}
