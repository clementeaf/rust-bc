use rust_bc::plugin_contracts::*;

#[test]
fn test_plugin_creation() {
    let plugin = Plugin::new(
        "plugin_1".to_string(),
        "Test Plugin".to_string(),
        "1.0.0".to_string(),
        "Author".to_string(),
        "A test plugin".to_string(),
    );

    assert_eq!(plugin.id, "plugin_1");
    assert_eq!(plugin.name, "Test Plugin");
    assert_eq!(plugin.version, "1.0.0");
    assert!(!plugin.enabled);
    assert_eq!(plugin.execution_count, 0);
}

#[test]
fn test_plugin_validation() {
    let plugin = Plugin::new(
        "plugin_1".to_string(),
        "Test Plugin".to_string(),
        "1.0.0".to_string(),
        "Author".to_string(),
        "A test plugin".to_string(),
    );

    assert!(plugin.validate().is_ok());
}

#[test]
fn test_plugin_validation_empty_id() {
    let plugin = Plugin::new(
        "".to_string(),
        "Test Plugin".to_string(),
        "1.0.0".to_string(),
        "Author".to_string(),
        "A test plugin".to_string(),
    );

    let result = plugin.validate();
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("ID cannot be empty"));
}

#[test]
fn test_plugin_validation_empty_name() {
    let plugin = Plugin::new(
        "plugin_1".to_string(),
        "".to_string(),
        "1.0.0".to_string(),
        "Author".to_string(),
        "A test plugin".to_string(),
    );

    let result = plugin.validate();
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("name cannot be empty"));
}

#[test]
fn test_plugin_validation_empty_version() {
    let plugin = Plugin::new(
        "plugin_1".to_string(),
        "Test Plugin".to_string(),
        "".to_string(),
        "Author".to_string(),
        "A test plugin".to_string(),
    );

    let result = plugin.validate();
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("version cannot be empty"));
}

#[test]
fn test_plugin_permissions() {
    let mut plugin = Plugin::new(
        "plugin_1".to_string(),
        "Test Plugin".to_string(),
        "1.0.0".to_string(),
        "Author".to_string(),
        "A test plugin".to_string(),
    );

    plugin.permissions.insert(PluginPermission::ReadBalance);

    assert!(plugin.has_permission(PluginPermission::ReadBalance));
    assert!(!plugin.has_permission(PluginPermission::WriteBalance));
}

#[test]
fn test_plugin_hooks() {
    let mut plugin = Plugin::new(
        "plugin_1".to_string(),
        "Test Plugin".to_string(),
        "1.0.0".to_string(),
        "Author".to_string(),
        "A test plugin".to_string(),
    );

    plugin.hooks.insert(HookPoint::BeforeTransfer);

    assert!(plugin.handles_hook(HookPoint::BeforeTransfer));
    assert!(!plugin.handles_hook(HookPoint::AfterTransfer));
}

#[test]
fn test_plugin_registry_creation() {
    let registry = PluginRegistry::new("owner".to_string(), 10);

    assert_eq!(registry.owner, "owner");
    assert_eq!(registry.max_plugins, 10);
    assert_eq!(registry.total_installed, 0);
}

#[test]
fn test_install_plugin() {
    let mut registry = PluginRegistry::new("owner".to_string(), 10);

    let plugin = Plugin::new(
        "plugin_1".to_string(),
        "Test Plugin".to_string(),
        "1.0.0".to_string(),
        "Author".to_string(),
        "A test plugin".to_string(),
    );

    let plugin_id = registry.install_plugin("owner", plugin, 100).unwrap();

    assert_eq!(plugin_id, "plugin_1");
    assert_eq!(registry.total_installed, 1);
    assert!(registry.get_plugin("plugin_1").is_some());
}

#[test]
fn test_install_plugin_not_owner() {
    let mut registry = PluginRegistry::new("owner".to_string(), 10);

    let plugin = Plugin::new(
        "plugin_1".to_string(),
        "Test Plugin".to_string(),
        "1.0.0".to_string(),
        "Author".to_string(),
        "A test plugin".to_string(),
    );

    let result = registry.install_plugin("not_owner", plugin, 100);

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Only owner can install"));
}

#[test]
fn test_install_plugin_duplicate() {
    let mut registry = PluginRegistry::new("owner".to_string(), 10);

    let plugin = Plugin::new(
        "plugin_1".to_string(),
        "Test Plugin".to_string(),
        "1.0.0".to_string(),
        "Author".to_string(),
        "A test plugin".to_string(),
    );

    registry.install_plugin("owner", plugin.clone(), 100).unwrap();

    let result = registry.install_plugin("owner", plugin, 100);

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("already exists"));
}

#[test]
fn test_install_plugin_max_reached() {
    let mut registry = PluginRegistry::new("owner".to_string(), 1);

    let plugin1 = Plugin::new(
        "plugin_1".to_string(),
        "Test Plugin 1".to_string(),
        "1.0.0".to_string(),
        "Author".to_string(),
        "A test plugin".to_string(),
    );

    let plugin2 = Plugin::new(
        "plugin_2".to_string(),
        "Test Plugin 2".to_string(),
        "1.0.0".to_string(),
        "Author".to_string(),
        "A test plugin".to_string(),
    );

    registry.install_plugin("owner", plugin1, 100).unwrap();

    let result = registry.install_plugin("owner", plugin2, 100);

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Maximum plugins"));
}

#[test]
fn test_enable_plugin() {
    let mut registry = PluginRegistry::new("owner".to_string(), 10);

    let plugin = Plugin::new(
        "plugin_1".to_string(),
        "Test Plugin".to_string(),
        "1.0.0".to_string(),
        "Author".to_string(),
        "A test plugin".to_string(),
    );

    registry.install_plugin("owner", plugin, 100).unwrap();
    registry.enable_plugin("owner", "plugin_1").unwrap();

    let plugin = registry.get_plugin("plugin_1").unwrap();
    assert!(plugin.enabled);
    assert_eq!(registry.enabled_plugins.len(), 1);
}

#[test]
fn test_enable_plugin_not_owner() {
    let mut registry = PluginRegistry::new("owner".to_string(), 10);

    let plugin = Plugin::new(
        "plugin_1".to_string(),
        "Test Plugin".to_string(),
        "1.0.0".to_string(),
        "Author".to_string(),
        "A test plugin".to_string(),
    );

    registry.install_plugin("owner", plugin, 100).unwrap();

    let result = registry.enable_plugin("not_owner", "plugin_1");

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Only owner can enable"));
}

#[test]
fn test_enable_plugin_already_enabled() {
    let mut registry = PluginRegistry::new("owner".to_string(), 10);

    let plugin = Plugin::new(
        "plugin_1".to_string(),
        "Test Plugin".to_string(),
        "1.0.0".to_string(),
        "Author".to_string(),
        "A test plugin".to_string(),
    );

    registry.install_plugin("owner", plugin, 100).unwrap();
    registry.enable_plugin("owner", "plugin_1").unwrap();

    let result = registry.enable_plugin("owner", "plugin_1");

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("already enabled"));
}

#[test]
fn test_disable_plugin() {
    let mut registry = PluginRegistry::new("owner".to_string(), 10);

    let plugin = Plugin::new(
        "plugin_1".to_string(),
        "Test Plugin".to_string(),
        "1.0.0".to_string(),
        "Author".to_string(),
        "A test plugin".to_string(),
    );

    registry.install_plugin("owner", plugin, 100).unwrap();
    registry.enable_plugin("owner", "plugin_1").unwrap();
    registry.disable_plugin("owner", "plugin_1").unwrap();

    let plugin = registry.get_plugin("plugin_1").unwrap();
    assert!(!plugin.enabled);
    assert_eq!(registry.enabled_plugins.len(), 0);
}

#[test]
fn test_disable_plugin_not_owner() {
    let mut registry = PluginRegistry::new("owner".to_string(), 10);

    let plugin = Plugin::new(
        "plugin_1".to_string(),
        "Test Plugin".to_string(),
        "1.0.0".to_string(),
        "Author".to_string(),
        "A test plugin".to_string(),
    );

    registry.install_plugin("owner", plugin, 100).unwrap();
    registry.enable_plugin("owner", "plugin_1").unwrap();

    let result = registry.disable_plugin("not_owner", "plugin_1");

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Only owner can disable"));
}

#[test]
fn test_uninstall_plugin() {
    let mut registry = PluginRegistry::new("owner".to_string(), 10);

    let plugin = Plugin::new(
        "plugin_1".to_string(),
        "Test Plugin".to_string(),
        "1.0.0".to_string(),
        "Author".to_string(),
        "A test plugin".to_string(),
    );

    registry.install_plugin("owner", plugin, 100).unwrap();
    registry.uninstall_plugin("owner", "plugin_1").unwrap();

    assert!(registry.get_plugin("plugin_1").is_none());
}

#[test]
fn test_uninstall_plugin_enabled() {
    let mut registry = PluginRegistry::new("owner".to_string(), 10);

    let plugin = Plugin::new(
        "plugin_1".to_string(),
        "Test Plugin".to_string(),
        "1.0.0".to_string(),
        "Author".to_string(),
        "A test plugin".to_string(),
    );

    registry.install_plugin("owner", plugin, 100).unwrap();
    registry.enable_plugin("owner", "plugin_1").unwrap();

    let result = registry.uninstall_plugin("owner", "plugin_1");

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Cannot uninstall enabled"));
}

#[test]
fn test_grant_permission() {
    let mut registry = PluginRegistry::new("owner".to_string(), 10);

    let plugin = Plugin::new(
        "plugin_1".to_string(),
        "Test Plugin".to_string(),
        "1.0.0".to_string(),
        "Author".to_string(),
        "A test plugin".to_string(),
    );

    registry.install_plugin("owner", plugin, 100).unwrap();
    registry
        .grant_permission("owner", "plugin_1", PluginPermission::ReadBalance)
        .unwrap();

    let plugin = registry.get_plugin("plugin_1").unwrap();
    assert!(plugin.has_permission(PluginPermission::ReadBalance));
}

#[test]
fn test_grant_permission_not_owner() {
    let mut registry = PluginRegistry::new("owner".to_string(), 10);

    let plugin = Plugin::new(
        "plugin_1".to_string(),
        "Test Plugin".to_string(),
        "1.0.0".to_string(),
        "Author".to_string(),
        "A test plugin".to_string(),
    );

    registry.install_plugin("owner", plugin, 100).unwrap();

    let result = registry.grant_permission("not_owner", "plugin_1", PluginPermission::ReadBalance);

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Only owner can grant"));
}

#[test]
fn test_revoke_permission() {
    let mut registry = PluginRegistry::new("owner".to_string(), 10);

    let plugin = Plugin::new(
        "plugin_1".to_string(),
        "Test Plugin".to_string(),
        "1.0.0".to_string(),
        "Author".to_string(),
        "A test plugin".to_string(),
    );

    registry.install_plugin("owner", plugin, 100).unwrap();
    registry
        .grant_permission("owner", "plugin_1", PluginPermission::ReadBalance)
        .unwrap();
    registry
        .revoke_permission("owner", "plugin_1", PluginPermission::ReadBalance)
        .unwrap();

    let plugin = registry.get_plugin("plugin_1").unwrap();
    assert!(!plugin.has_permission(PluginPermission::ReadBalance));
}

#[test]
fn test_register_hook() {
    let mut registry = PluginRegistry::new("owner".to_string(), 10);

    let plugin = Plugin::new(
        "plugin_1".to_string(),
        "Test Plugin".to_string(),
        "1.0.0".to_string(),
        "Author".to_string(),
        "A test plugin".to_string(),
    );

    registry.install_plugin("owner", plugin, 100).unwrap();
    registry
        .register_hook("owner", "plugin_1", HookPoint::BeforeTransfer)
        .unwrap();

    let plugin = registry.get_plugin("plugin_1").unwrap();
    assert!(plugin.handles_hook(HookPoint::BeforeTransfer));
}

#[test]
fn test_register_hook_enabled_plugin() {
    let mut registry = PluginRegistry::new("owner".to_string(), 10);

    let plugin = Plugin::new(
        "plugin_1".to_string(),
        "Test Plugin".to_string(),
        "1.0.0".to_string(),
        "Author".to_string(),
        "A test plugin".to_string(),
    );

    registry.install_plugin("owner", plugin, 100).unwrap();
    registry.enable_plugin("owner", "plugin_1").unwrap();
    registry
        .register_hook("owner", "plugin_1", HookPoint::BeforeTransfer)
        .unwrap();

    let plugins = registry.get_plugins_for_hook(HookPoint::BeforeTransfer);
    assert_eq!(plugins.len(), 1);
    assert_eq!(plugins[0].id, "plugin_1");
}

#[test]
fn test_unregister_hook() {
    let mut registry = PluginRegistry::new("owner".to_string(), 10);

    let plugin = Plugin::new(
        "plugin_1".to_string(),
        "Test Plugin".to_string(),
        "1.0.0".to_string(),
        "Author".to_string(),
        "A test plugin".to_string(),
    );

    registry.install_plugin("owner", plugin, 100).unwrap();
    registry
        .register_hook("owner", "plugin_1", HookPoint::BeforeTransfer)
        .unwrap();
    registry
        .unregister_hook("owner", "plugin_1", HookPoint::BeforeTransfer)
        .unwrap();

    let plugin = registry.get_plugin("plugin_1").unwrap();
    assert!(!plugin.handles_hook(HookPoint::BeforeTransfer));
}

#[test]
fn test_execute_hook() {
    let mut registry = PluginRegistry::new("owner".to_string(), 10);

    let mut plugin = Plugin::new(
        "plugin_1".to_string(),
        "Test Plugin".to_string(),
        "1.0.0".to_string(),
        "Author".to_string(),
        "A test plugin".to_string(),
    );

    plugin.hooks.insert(HookPoint::BeforeTransfer);
    registry.install_plugin("owner", plugin, 100).unwrap();
    registry.enable_plugin("owner", "plugin_1").unwrap();

    let results = registry.execute_hook(HookPoint::BeforeTransfer, "test_context");

    assert_eq!(results.len(), 1);
    assert!(results[0].success);
    assert!(results[0].message.contains("executed"));

    let plugin = registry.get_plugin("plugin_1").unwrap();
    assert_eq!(plugin.execution_count, 1);
}

#[test]
fn test_execute_hook_multiple_plugins() {
    let mut registry = PluginRegistry::new("owner".to_string(), 10);

    let mut plugin1 = Plugin::new(
        "plugin_1".to_string(),
        "Test Plugin 1".to_string(),
        "1.0.0".to_string(),
        "Author".to_string(),
        "A test plugin".to_string(),
    );

    plugin1.hooks.insert(HookPoint::BeforeTransfer);

    let mut plugin2 = Plugin::new(
        "plugin_2".to_string(),
        "Test Plugin 2".to_string(),
        "1.0.0".to_string(),
        "Author".to_string(),
        "A test plugin".to_string(),
    );

    plugin2.hooks.insert(HookPoint::BeforeTransfer);

    registry.install_plugin("owner", plugin1, 100).unwrap();
    registry.install_plugin("owner", plugin2, 100).unwrap();
    registry.enable_plugin("owner", "plugin_1").unwrap();
    registry.enable_plugin("owner", "plugin_2").unwrap();

    let results = registry.execute_hook(HookPoint::BeforeTransfer, "test_context");

    assert_eq!(results.len(), 2);
}

#[test]
fn test_get_all_plugins() {
    let mut registry = PluginRegistry::new("owner".to_string(), 10);

    let plugin1 = Plugin::new(
        "plugin_1".to_string(),
        "Test Plugin 1".to_string(),
        "1.0.0".to_string(),
        "Author".to_string(),
        "A test plugin".to_string(),
    );

    let plugin2 = Plugin::new(
        "plugin_2".to_string(),
        "Test Plugin 2".to_string(),
        "1.0.0".to_string(),
        "Author".to_string(),
        "A test plugin".to_string(),
    );

    registry.install_plugin("owner", plugin1, 100).unwrap();
    registry.install_plugin("owner", plugin2, 100).unwrap();

    let plugins = registry.get_all_plugins();

    assert_eq!(plugins.len(), 2);
}

#[test]
fn test_get_enabled_plugins() {
    let mut registry = PluginRegistry::new("owner".to_string(), 10);

    let plugin1 = Plugin::new(
        "plugin_1".to_string(),
        "Test Plugin 1".to_string(),
        "1.0.0".to_string(),
        "Author".to_string(),
        "A test plugin".to_string(),
    );

    let plugin2 = Plugin::new(
        "plugin_2".to_string(),
        "Test Plugin 2".to_string(),
        "1.0.0".to_string(),
        "Author".to_string(),
        "A test plugin".to_string(),
    );

    registry.install_plugin("owner", plugin1, 100).unwrap();
    registry.install_plugin("owner", plugin2, 100).unwrap();
    registry.enable_plugin("owner", "plugin_1").unwrap();

    let plugins = registry.get_enabled_plugins();

    assert_eq!(plugins.len(), 1);
    assert_eq!(plugins[0].id, "plugin_1");
}

#[test]
fn test_get_plugins_for_hook() {
    let mut registry = PluginRegistry::new("owner".to_string(), 10);

    let mut plugin1 = Plugin::new(
        "plugin_1".to_string(),
        "Test Plugin 1".to_string(),
        "1.0.0".to_string(),
        "Author".to_string(),
        "A test plugin".to_string(),
    );

    plugin1.hooks.insert(HookPoint::BeforeTransfer);

    let mut plugin2 = Plugin::new(
        "plugin_2".to_string(),
        "Test Plugin 2".to_string(),
        "1.0.0".to_string(),
        "Author".to_string(),
        "A test plugin".to_string(),
    );

    plugin2.hooks.insert(HookPoint::AfterTransfer);

    registry.install_plugin("owner", plugin1, 100).unwrap();
    registry.install_plugin("owner", plugin2, 100).unwrap();
    registry.enable_plugin("owner", "plugin_1").unwrap();
    registry.enable_plugin("owner", "plugin_2").unwrap();

    let plugins = registry.get_plugins_for_hook(HookPoint::BeforeTransfer);

    assert_eq!(plugins.len(), 1);
    assert_eq!(plugins[0].id, "plugin_1");
}

#[test]
fn test_get_statistics() {
    let mut registry = PluginRegistry::new("owner".to_string(), 10);

    let mut plugin1 = Plugin::new(
        "plugin_1".to_string(),
        "Test Plugin 1".to_string(),
        "1.0.0".to_string(),
        "Author".to_string(),
        "A test plugin".to_string(),
    );

    plugin1.hooks.insert(HookPoint::BeforeTransfer);

    let plugin2 = Plugin::new(
        "plugin_2".to_string(),
        "Test Plugin 2".to_string(),
        "1.0.0".to_string(),
        "Author".to_string(),
        "A test plugin".to_string(),
    );

    registry.install_plugin("owner", plugin1, 100).unwrap();
    registry.install_plugin("owner", plugin2, 100).unwrap();
    registry.enable_plugin("owner", "plugin_1").unwrap();
    registry.execute_hook(HookPoint::BeforeTransfer, "context");

    let stats = registry.get_statistics();

    assert_eq!(stats.total_installed, 2);
    assert_eq!(stats.total_plugins, 2);
    assert_eq!(stats.enabled_plugins, 1);
    assert_eq!(stats.disabled_plugins, 1);
    assert_eq!(stats.hooks_registered, 1);
    assert_eq!(stats.total_hook_executions, 1);
}

#[test]
fn test_plugin_context() {
    let mut context = PluginContext::new("0x123".to_string(), "user".to_string(), 100);

    context.add_data("key1".to_string(), "value1".to_string());

    assert_eq!(context.get_data("key1"), Some("value1".to_string()));
    assert_eq!(context.get_data("key2"), None);
}

#[test]
fn test_hook_result_success() {
    let result = HookResult::success("Operation successful".to_string());

    assert!(result.success);
    assert_eq!(result.message, "Operation successful");
    assert!(result.data.is_none());
}

#[test]
fn test_hook_result_failure() {
    let result = HookResult::failure("Operation failed".to_string());

    assert!(!result.success);
    assert_eq!(result.message, "Operation failed");
}

#[test]
fn test_hook_result_with_data() {
    let result = HookResult::with_data(true, "Success".to_string(), "data_value".to_string());

    assert!(result.success);
    assert_eq!(result.message, "Success");
    assert_eq!(result.data, Some("data_value".to_string()));
}

#[test]
fn test_multiple_hooks_on_plugin() {
    let mut registry = PluginRegistry::new("owner".to_string(), 10);

    let mut plugin = Plugin::new(
        "plugin_1".to_string(),
        "Test Plugin".to_string(),
        "1.0.0".to_string(),
        "Author".to_string(),
        "A test plugin".to_string(),
    );

    plugin.hooks.insert(HookPoint::BeforeTransfer);
    plugin.hooks.insert(HookPoint::AfterTransfer);
    plugin.hooks.insert(HookPoint::BeforeMint);

    registry.install_plugin("owner", plugin, 100).unwrap();
    registry.enable_plugin("owner", "plugin_1").unwrap();

    let stats = registry.get_statistics();

    assert_eq!(stats.hooks_registered, 3);
}

#[test]
fn test_multiple_permissions_on_plugin() {
    let mut registry = PluginRegistry::new("owner".to_string(), 10);

    let plugin = Plugin::new(
        "plugin_1".to_string(),
        "Test Plugin".to_string(),
        "1.0.0".to_string(),
        "Author".to_string(),
        "A test plugin".to_string(),
    );

    registry.install_plugin("owner", plugin, 100).unwrap();

    registry
        .grant_permission("owner", "plugin_1", PluginPermission::ReadBalance)
        .unwrap();
    registry
        .grant_permission("owner", "plugin_1", PluginPermission::WriteBalance)
        .unwrap();
    registry
        .grant_permission("owner", "plugin_1", PluginPermission::ReadMetadata)
        .unwrap();

    let plugin = registry.get_plugin("plugin_1").unwrap();

    assert!(plugin.has_permission(PluginPermission::ReadBalance));
    assert!(plugin.has_permission(PluginPermission::WriteBalance));
    assert!(plugin.has_permission(PluginPermission::ReadMetadata));
    assert!(!plugin.has_permission(PluginPermission::Custom));
}

#[test]
fn test_disable_and_enable_updates_hooks() {
    let mut registry = PluginRegistry::new("owner".to_string(), 10);

    let mut plugin = Plugin::new(
        "plugin_1".to_string(),
        "Test Plugin".to_string(),
        "1.0.0".to_string(),
        "Author".to_string(),
        "A test plugin".to_string(),
    );

    plugin.hooks.insert(HookPoint::BeforeTransfer);

    registry.install_plugin("owner", plugin, 100).unwrap();
    registry.enable_plugin("owner", "plugin_1").unwrap();

    let plugins_before = registry.get_plugins_for_hook(HookPoint::BeforeTransfer);
    assert_eq!(plugins_before.len(), 1);

    registry.disable_plugin("owner", "plugin_1").unwrap();

    let plugins_after_disable = registry.get_plugins_for_hook(HookPoint::BeforeTransfer);
    assert_eq!(plugins_after_disable.len(), 0);

    registry.enable_plugin("owner", "plugin_1").unwrap();

    let plugins_after_enable = registry.get_plugins_for_hook(HookPoint::BeforeTransfer);
    assert_eq!(plugins_after_enable.len(), 1);
}
