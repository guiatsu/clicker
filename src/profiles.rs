use std::{env, fs, io, path::PathBuf};

use serde::{Deserialize, Serialize};

use crate::settings::Settings;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub name: String,
    pub settings: Settings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ProfileStore {
    pub active_profile: String,
    pub profiles: Vec<Profile>,
}

impl Default for ProfileStore {
    fn default() -> Self {
        Self {
            active_profile: "Padrão".to_owned(),
            profiles: vec![Profile {
                name: "Padrão".to_owned(),
                settings: Settings::default(),
            }],
        }
    }
}

impl ProfileStore {
    pub fn active(&self) -> &Profile {
        self.profiles
            .iter()
            .find(|profile| profile.name == self.active_profile)
            .unwrap_or(&self.profiles[0])
    }

    pub fn active_mut(&mut self) -> &mut Profile {
        let index = self
            .profiles
            .iter()
            .position(|profile| profile.name == self.active_profile)
            .unwrap_or(0);
        &mut self.profiles[index]
    }

    pub fn save(&self) -> io::Result<()> {
        let path = settings_path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let bytes = serde_json::to_vec_pretty(self).map_err(io::Error::other)?;
        let temporary_path = path.with_extension("json.tmp");
        fs::write(&temporary_path, bytes)?;
        fs::rename(temporary_path, path)
    }

    pub fn load() -> io::Result<Self> {
        let path = settings_path()?;
        let contents = match fs::read(&path) {
            Ok(contents) => contents,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                let legacy = legacy_settings_path()?;
                if legacy == path {
                    return Err(error);
                }
                fs::read(legacy)?
            }
            Err(error) => return Err(error),
        };
        let mut store: Self = serde_json::from_slice(&contents).map_err(io::Error::other)?;
        store.normalize();
        Ok(store)
    }

    fn normalize(&mut self) {
        self.profiles
            .retain(|profile| !profile.name.trim().is_empty());
        if self.profiles.is_empty() {
            *self = Self::default();
            return;
        }
        if !self
            .profiles
            .iter()
            .any(|profile| profile.name == self.active_profile)
        {
            self.active_profile = self.profiles[0].name.clone();
        }
    }

    pub fn create(&mut self, name: String, duplicate_active: bool) -> Result<(), String> {
        let name = name.trim();
        if name.is_empty() {
            return Err("Informe um nome para o perfil.".to_owned());
        }
        if self
            .profiles
            .iter()
            .any(|profile| profile.name.eq_ignore_ascii_case(name))
        {
            return Err("Já existe um perfil com esse nome.".to_owned());
        }
        let settings = if duplicate_active {
            self.active().settings.clone()
        } else {
            Settings::default()
        };
        let name = name.to_owned();
        self.profiles.push(Profile {
            name: name.clone(),
            settings,
        });
        self.active_profile = name;
        Ok(())
    }

    pub fn rename_active(&mut self, name: String) -> Result<(), String> {
        let name = name.trim();
        if name.is_empty() {
            return Err("Informe um nome para o perfil.".to_owned());
        }
        if self.profiles.iter().any(|profile| {
            profile.name != self.active_profile && profile.name.eq_ignore_ascii_case(name)
        }) {
            return Err("Já existe um perfil com esse nome.".to_owned());
        }
        let old_name = self.active_profile.clone();
        let profile = self
            .profiles
            .iter_mut()
            .find(|profile| profile.name == old_name)
            .ok_or_else(|| "O perfil ativo não existe.".to_owned())?;
        profile.name = name.to_owned();
        self.active_profile = name.to_owned();
        Ok(())
    }

    pub fn delete_active(&mut self) -> Result<(), String> {
        if self.profiles.len() == 1 {
            return Err("Mantenha ao menos um perfil.".to_owned());
        }
        let old_name = self.active_profile.clone();
        self.profiles.retain(|profile| profile.name != old_name);
        self.active_profile = self.profiles[0].name.clone();
        Ok(())
    }
}

pub fn settings_path() -> io::Result<PathBuf> {
    #[cfg(windows)]
    let base = env::var_os("APPDATA").map(PathBuf::from);
    #[cfg(not(windows))]
    let base = env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")));

    base.map(|base| base.join("NeoAutoClicker").join("profiles.json"))
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                "diretório de configuração indisponível",
            )
        })
}

fn legacy_settings_path() -> io::Result<PathBuf> {
    #[cfg(windows)]
    let base = env::var_os("APPDATA").map(PathBuf::from);
    #[cfg(not(windows))]
    let base = env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")));

    base.map(|base| base.join("RustAutoClicker").join("profiles.json"))
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                "diretório de configuração indisponível",
            )
        })
}

pub fn load_or_default() -> (ProfileStore, Option<String>) {
    match ProfileStore::load() {
        Ok(store) => (store, None),
        Err(error) if error.kind() == io::ErrorKind::NotFound => (ProfileStore::default(), None),
        Err(error) => (
            ProfileStore::default(),
            Some(format!(
                "Não foi possível carregar os perfis; usando o padrão: {error}"
            )),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_lifecycle_preserves_settings_when_duplicated() {
        let mut store = ProfileStore::default();
        store.active_mut().settings.cps = 42.0;
        store.create("Jogo".to_owned(), true).unwrap();
        assert_eq!(store.active().settings.cps, 42.0);
        store.rename_active("Jogo 2".to_owned()).unwrap();
        assert_eq!(store.active_profile, "Jogo 2");
        store.delete_active().unwrap();
        assert_eq!(store.profiles.len(), 1);
    }

    #[test]
    fn serialized_profiles_round_trip() {
        let store = ProfileStore::default();
        let bytes = serde_json::to_vec(&store).unwrap();
        let restored: ProfileStore = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(restored.active_profile, store.active_profile);
        assert_eq!(restored.active().settings.cps, 10.0);
    }
}
