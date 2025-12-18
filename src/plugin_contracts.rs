/**
 * Plugin/Extension System for Smart Contracts
 *
 * Allow contracts to extend functionality:
 * - Plugin registry in contract state
 * - Hooks system (before/after transfer, etc.)
 * - Plugin permissions (what state can access)
 */

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Plugin permissions - what state data can a plugin access/modify
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum PluginPermission {
    ReadBalance,
    WriteBalance,
    ReadMetadata,
    WriteMetadata,
    ReadAllowance,
    WriteAllowance,
    Custom,
}

/// Hook execution point in contract lifecycle
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum HookPoint {
    BeforeTransfer,
    AfterTransfer,
    BeforeMint,
    AfterMint,
    BeforeBurn,
    AfterBurn,
    BeforeApprove,
    AfterApprove,
    Custom,
}

/// Plugin hook execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookResult {
    pub success: bool,
    pub message: String,
    pub data: Option<String>,
}

impl HookResult {
    pub fn success(message: String) -> Self {
        HookResult {
            success: true,
            message,
            data: None,
        }
    }

    pub fn failure(message: String) -> Self {
        HookResult {
            success: false,
            message,
            data: None,
        }
    }

    pub fn with_data(success: bool, message: String, data: String) -> Self {
        HookResult {
            success,
            message,
            data: Some(data),
        }
    }
}

/// Plugin definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plugin {
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    pub enabled: bool,
    pub permissions: HashSet<PluginPermission>,
    pub hooks: HashSet<HookPoint>,
    pub installed_at_block: u64,
    pub execution_count: u64,
}

impl Plugin {
    pub fn new(
        id: String,
        name: String,
        version: String,
        author: String,
        description: String,
    ) -> Self {
        Plugin {
            id,
            name,
            version,
            author,
            description,
            enabled: false,
            permissions: HashSet::new(),
            hooks: HashSet::new(),
            installed_at_block: 0,
            execution_count: 0,
        }
    }

    /// Check if plugin has a specific permission
    pub fn has_permission(&self, permission: PluginPermission) -> bool {
        self.permissions.contains(&permission)
    }

    /// Check if plugin handles a specific hook
    pub fn handles_hook(&self, hook: HookPoint) -> bool {
        self.hooks.contains(&hook)
    }

    /// Validate plugin configuration
    pub fn validate(&self) -> Result<(), String> {
        if self.id.is_empty() {
            return Err("Plugin ID cannot be empty".to_string());
        }

        if self.name.is_empty() {
            return Err("Plugin name cannot be empty".to_string());
        }

        if self.version.is_empty() {
            return Err("Plugin version cannot be empty".to_string());
        }

        Ok(())
    }
}

/// Plugin registry and manager
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginRegistry {
    pub owner: String,
    pub plugins: HashMap<String, Plugin>,
    pub enabled_plugins: Vec<String>,
    pub hooks: HashMap<HookPoint, Vec<String>>, // Maps hooks to plugin IDs
    pub max_plugins: u64,
    pub total_installed: u64,
}

impl PluginRegistry {
    pub fn new(owner: String, max_plugins: u64) -> Self {
        PluginRegistry {
            owner,
            plugins: HashMap::new(),
            enabled_plugins: Vec::new(),
            hooks: HashMap::new(),
            max_plugins,
            total_installed: 0,
        }
    }

    /// Install a new plugin (owner only)
    pub fn install_plugin(
        &mut self,
        caller: &str,
        mut plugin: Plugin,
        current_block: u64,
    ) -> Result<String, String> {
        if caller != self.owner {
            return Err("Only owner can install plugins".to_string());
        }

        plugin.validate()?;

        if self.plugins.contains_key(&plugin.id) {
            return Err(format!("Plugin with ID {} already exists", plugin.id));
        }

        if self.plugins.len() >= self.max_plugins as usize {
            return Err(format!("Maximum plugins ({}) reached", self.max_plugins));
        }

        plugin.installed_at_block = current_block;
        let plugin_id = plugin.id.clone();
        self.plugins.insert(plugin_id.clone(), plugin);
        self.total_installed += 1;

        Ok(plugin_id)
    }

    /// Enable a plugin (owner only)
    pub fn enable_plugin(&mut self, caller: &str, plugin_id: &str) -> Result<(), String> {
        if caller != self.owner {
            return Err("Only owner can enable plugins".to_string());
        }

        let plugin = self
            .plugins
            .get_mut(plugin_id)
            .ok_or("Plugin not found")?;

        if plugin.enabled {
            return Err("Plugin already enabled".to_string());
        }

        plugin.enabled = true;
        self.enabled_plugins.push(plugin_id.to_string());

        // Register hooks
        for hook in &plugin.hooks {
            self.hooks
                .entry(*hook)
                .or_default()
                .push(plugin_id.to_string());
        }

        Ok(())
    }

    /// Disable a plugin (owner only)
    pub fn disable_plugin(&mut self, caller: &str, plugin_id: &str) -> Result<(), String> {
        if caller != self.owner {
            return Err("Only owner can disable plugins".to_string());
        }

        let plugin = self
            .plugins
            .get_mut(plugin_id)
            .ok_or("Plugin not found")?;

        if !plugin.enabled {
            return Err("Plugin already disabled".to_string());
        }

        plugin.enabled = false;
        self.enabled_plugins.retain(|id| id != plugin_id);

        // Unregister hooks
        for hook in &plugin.hooks {
            if let Some(plugins) = self.hooks.get_mut(hook) {
                plugins.retain(|id| id != plugin_id);
            }
        }

        Ok(())
    }

    /// Uninstall a plugin (owner only)
    pub fn uninstall_plugin(&mut self, caller: &str, plugin_id: &str) -> Result<(), String> {
        if caller != self.owner {
            return Err("Only owner can uninstall plugins".to_string());
        }

        let plugin = self
            .plugins
            .get(plugin_id)
            .ok_or("Plugin not found")?;

        if plugin.enabled {
            return Err("Cannot uninstall enabled plugin - disable first".to_string());
        }

        self.plugins.remove(plugin_id);
        Ok(())
    }

    /// Execute hook for all registered plugins
    pub fn execute_hook(&mut self, hook: HookPoint, context: &str) -> Vec<HookResult> {
        let mut results = Vec::new();

        if let Some(plugin_ids) = self.hooks.get(&hook).cloned() {
            for plugin_id in plugin_ids {
                if let Some(plugin) = self.plugins.get_mut(&plugin_id) {
                    if plugin.enabled {
                        plugin.execution_count += 1;
                        let result = HookResult::success(format!(
                            "Plugin {} executed for hook {:?} with context: {}",
                            plugin_id, hook, context
                        ));
                        results.push(result);
                    }
                }
            }
        }

        results
    }

    /// Add permission to plugin (owner only)
    pub fn grant_permission(
        &mut self,
        caller: &str,
        plugin_id: &str,
        permission: PluginPermission,
    ) -> Result<(), String> {
        if caller != self.owner {
            return Err("Only owner can grant permissions".to_string());
        }

        let plugin = self
            .plugins
            .get_mut(plugin_id)
            .ok_or("Plugin not found")?;

        plugin.permissions.insert(permission);
        Ok(())
    }

    /// Remove permission from plugin (owner only)
    pub fn revoke_permission(
        &mut self,
        caller: &str,
        plugin_id: &str,
        permission: PluginPermission,
    ) -> Result<(), String> {
        if caller != self.owner {
            return Err("Only owner can revoke permissions".to_string());
        }

        let plugin = self
            .plugins
            .get_mut(plugin_id)
            .ok_or("Plugin not found")?;

        plugin.permissions.remove(&permission);
        Ok(())
    }

    /// Register plugin to handle a hook (owner only)
    pub fn register_hook(
        &mut self,
        caller: &str,
        plugin_id: &str,
        hook: HookPoint,
    ) -> Result<(), String> {
        if caller != self.owner {
            return Err("Only owner can register hooks".to_string());
        }

        let plugin = self
            .plugins
            .get_mut(plugin_id)
            .ok_or("Plugin not found")?;

        plugin.hooks.insert(hook);

        // Update hook registry if plugin is enabled
        if plugin.enabled {
            self.hooks
                .entry(hook)
                .or_default()
                .push(plugin_id.to_string());
        }

        Ok(())
    }

    /// Unregister hook from plugin (owner only)
    pub fn unregister_hook(
        &mut self,
        caller: &str,
        plugin_id: &str,
        hook: HookPoint,
    ) -> Result<(), String> {
        if caller != self.owner {
            return Err("Only owner can unregister hooks".to_string());
        }

        let plugin = self
            .plugins
            .get_mut(plugin_id)
            .ok_or("Plugin not found")?;

        plugin.hooks.remove(&hook);

        // Update hook registry
        if let Some(plugins) = self.hooks.get_mut(&hook) {
            plugins.retain(|id| id != plugin_id);
        }

        Ok(())
    }

    /// Get plugin details
    pub fn get_plugin(&self, plugin_id: &str) -> Option<Plugin> {
        self.plugins.get(plugin_id).cloned()
    }

    /// Get all plugins
    pub fn get_all_plugins(&self) -> Vec<Plugin> {
        self.plugins.values().cloned().collect()
    }

    /// Get enabled plugins
    pub fn get_enabled_plugins(&self) -> Vec<Plugin> {
        self.enabled_plugins
            .iter()
            .filter_map(|id| self.plugins.get(id).cloned())
            .collect()
    }

    /// Get plugins for a specific hook
    pub fn get_plugins_for_hook(&self, hook: HookPoint) -> Vec<Plugin> {
        if let Some(plugin_ids) = self.hooks.get(&hook) {
            plugin_ids
                .iter()
                .filter_map(|id| self.plugins.get(id).cloned())
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Get registry statistics
    pub fn get_statistics(&self) -> PluginStatistics {
        let total_enabled = self.enabled_plugins.len() as u64;
        let total_disabled = (self.plugins.len() as u64) - total_enabled;
        let total_execution_count: u64 = self.plugins.values().map(|p| p.execution_count).sum();

        PluginStatistics {
            total_installed: self.total_installed,
            total_plugins: self.plugins.len() as u64,
            enabled_plugins: total_enabled,
            disabled_plugins: total_disabled,
            hooks_registered: self.hooks.len() as u64,
            total_hook_executions: total_execution_count,
        }
    }
}

/// Plugin registry statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginStatistics {
    pub total_installed: u64,
    pub total_plugins: u64,
    pub enabled_plugins: u64,
    pub disabled_plugins: u64,
    pub hooks_registered: u64,
    pub total_hook_executions: u64,
}

/// Plugin configuration context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginContext {
    pub contract_address: String,
    pub caller: String,
    pub current_block: u64,
    pub data: HashMap<String, String>,
}

impl PluginContext {
    pub fn new(contract_address: String, caller: String, current_block: u64) -> Self {
        PluginContext {
            contract_address,
            caller,
            current_block,
            data: HashMap::new(),
        }
    }

    /// Add contextual data
    pub fn add_data(&mut self, key: String, value: String) {
        self.data.insert(key, value);
    }

    /// Get contextual data
    pub fn get_data(&self, key: &str) -> Option<String> {
        self.data.get(key).cloned()
    }
}
