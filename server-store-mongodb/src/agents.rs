use sda_protocol::*;
use sda_server::stores;
use sda_server::errors::*;
use {to_bson, to_doc, Dao};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct AgentDocument {
    id: AgentId,
    agent: Agent,
    profile: Option<Profile>,
    #[serde(default)]
    keys: Vec<Labelled<EncryptionKeyId, SignedEncryptionKey>>,
}

pub struct MongoAgentsStore(Dao<AgentId, AgentDocument>);

impl MongoAgentsStore {
    pub fn new(db: &::mongodb::db::Database) -> SdaServerResult<MongoAgentsStore> {
        use mongodb::db::ThreadedDatabase;
        let dao = Dao::new(db.collection("agents"));
        dao.ensure_index(d!("id" => 1), true)?;
        Ok(MongoAgentsStore(dao))
    }
}

impl stores::BaseStore for MongoAgentsStore {
    fn ping(&self) -> SdaServerResult<()> {
        self.0.ping()
    }
}

impl stores::AgentsStore for MongoAgentsStore {
    fn create_agent(&self, agent: &Agent) -> SdaServerResult<()> {
        self.0.modisert_by_id(&agent.id, d!("$set" => d! ( "agent" => to_doc(agent)?) ))
    }

    fn get_agent(&self, id: &AgentId) -> SdaServerResult<Option<Agent>> {
        self.0
            .get_by_id(id)
            .map(|opt| opt.map(|ad| ad.agent))
    }

    fn upsert_profile(&self, profile: &Profile) -> SdaServerResult<()> {
        self.0.modify_by_id(&profile.owner,
                            d!("$set" => d!("profile" => to_doc(profile)?)))
    }

    fn get_profile(&self, owner: &AgentId) -> SdaServerResult<Option<Profile>> {
        self.0
            .get_by_id(owner)
            .map(|opt| opt.and_then(|ad| ad.profile))
    }

    fn create_encryption_key(&self, key: &SignedEncryptionKey) -> SdaServerResult<()> {
        self.0
            .modify_by_id(&key.signer,
                          d!("$pull" => d!("keys" => d!("id" => to_bson(key.id())?))))?;
        self.0.modify_by_id(&key.signer,
                            d!("$push" => d!("keys" => to_doc(&label(key.id(), key))?)))
    }

    fn get_encryption_key(&self,
                          key: &EncryptionKeyId)
                          -> SdaServerResult<Option<SignedEncryptionKey>> {
        let selector = d!("keys.id" => to_bson(key)?);
        self.0.get(selector).map(|opt| {
            opt.and_then(|ad| ad.keys.into_iter().find(|k| k.id == *key))
                .map(|k| k.body)
        })
    }

    fn suggest_committee(&self) -> SdaServerResult<Vec<ClerkCandidate>> {
        self.0
            .find(d!())?
            .map(|res| {
                res.map(|ad| {
                    ClerkCandidate {
                        id: ad.id,
                        keys: ad.keys.iter().map(|it| it.id).collect(),
                    }
                })
            })
            .collect()
    }
}
