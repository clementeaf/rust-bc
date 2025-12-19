# 05: Target Frontend Architecture

**Phase 1 Day 3 - Task 2**  
**Status**: Design Complete  
**Scope**: Client-tier using NeuroAccessMaui patterns  
**Principles**: Clean MVVM, zero business logic in views, service-oriented  

---

## 1. Architecture Philosophy: MVVM with Clear Responsibilities

### Design Principle
**Model ≠ ViewModel ≠ View** → Each layer has ONE purpose → Easy to test, reuse, evolve

```
┌──────────────────────────────────┐
│     Presentation Layer           │ ← .NET MAUI XAML Views
│  (INotifyPropertyChanged bound)  │
├──────────────────────────────────┤
│     ViewModel Layer              │ ← Presentation Logic (MVVM)
│  (Orchestrates UI state)         │
├──────────────────────────────────┤
│     Service Layer                │ ← Business Logic (HTTP, Auth, etc.)
│  (Implements domain operations)  │
├──────────────────────────────────┤
│     Model Layer                  │ ← Domain Objects
│  (Data contracts, immutable)     │
├──────────────────────────────────┤
│     Persistence Layer            │ ← Local SQLite DB
│  (Encrypted local storage)       │
└──────────────────────────────────┘
  ↓ Dependency Injection (bottom-up)
```

### Separation Rules (CRITICAL)
- **View**: XAML only, zero code-behind logic
- **ViewModel**: UI state orchestration ONLY, no HTTP calls
- **Service**: Business logic, HTTP calls, transformations
- **Model**: Data contracts, validation rules, immutable DTOs
- **Persistence**: Local storage, encryption, migrations

**Violation = UI logic leakage**

---

## 2. Layer 1: Persistence Layer (Foundation)

**Responsibility**: Encrypted local database, sync state, offline capability

### 2.1 Local Database Design

```csharp
// File: Features/Persistence/LocalDatabase.cs

public interface ILocalDatabase
{
    Task<T> GetAsync<T>(string id) where T : class;
    Task<List<T>> QueryAsync<T>(Expression<Func<T, bool>> predicate) where T : class;
    Task InsertAsync<T>(T entity) where T : class;
    Task UpdateAsync<T>(T entity) where T : class;
    Task DeleteAsync<T>(string id) where T : class;
}

public class EncryptedSqliteDatabase : ILocalDatabase
{
    private readonly SQLiteAsyncConnection _connection;
    private readonly IEncryptionService _encryption;
    
    public EncryptedSqliteDatabase(string dbPath, IEncryptionService encryption)
    {
        _encryption = encryption;
        var connectionString = encryption.EncryptConnectionString(dbPath);
        _connection = new SQLiteAsyncConnection(connectionString);
    }
    
    public async Task InsertAsync<T>(T entity) where T : class
    {
        var json = JsonSerializer.Serialize(entity);
        var encrypted = await _encryption.EncryptAsync(json);
        
        await _connection.InsertAsync(new LocalRecord
        {
            Id = GetIdFromEntity(entity),
            TypeName = typeof(T).Name,
            EncryptedData = encrypted,
            CreatedAt = DateTime.UtcNow,
        });
    }
}
```

**Key Separation**: 
- Database layer knows ONLY about persistence
- Business logic never directly touches DB
- Encryption is transparent to consumers

### 2.2 Sync State Management

```csharp
// File: Features/Persistence/SyncState.cs

public class SyncState
{
    public string EntityId { get; set; }
    public string EntityType { get; set; }
    public SyncStatus Status { get; set; }  // Pending, Syncing, Synced, Error
    public DateTime LastAttempt { get; set; }
    public string ErrorMessage { get; set; }
}

public enum SyncStatus
{
    Pending = 0,
    Syncing = 1,
    Synced = 2,
    Error = 3,
    Conflict = 4,  // Server has newer version
}

public interface ISyncStateService
{
    Task MarkForSyncAsync<T>(T entity) where T : class;
    Task MarkSyncedAsync<T>(T entity) where T : class;
    Task MarkErrorAsync<T>(T entity, string error) where T : class;
    Task<List<SyncState>> GetPendingSyncAsync();
}
```

**Key**: Separate concern = sync state independent of entity

---

## 3. Layer 2: Model Layer (Domain Objects)

**Responsibility**: Data contracts, validation, immutable structures

### 3.1 Domain Models

```csharp
// File: Features/Domain/Models.cs

[Immutable]
public record DigitalIdentity(
    string Id,
    string DID,
    string PublicKey,
    List<Credential> Credentials,
    IdentityStatus Status,
    DateTime CreatedAt,
    DateTime UpdatedAt
);

[Immutable]
public record Credential(
    string Id,
    string IssuerId,
    string SubjectId,
    Dictionary<string, object> Claims,
    string Signature,
    DateTime IssuedAt,
    DateTime? ExpiresAt
)
{
    public bool IsExpired => ExpiresAt.HasValue && ExpiresAt < DateTime.UtcNow;
    public bool IsValid => !IsExpired;
}

[Immutable]
public record Transaction(
    string Id,
    List<TransactionInput> Inputs,
    List<TransactionOutput> Outputs,
    string Signature,
    DateTime CreatedAt,
    TransactionStatus Status
)
{
    public decimal TotalInput => Inputs.Sum(i => i.Amount);
    public decimal TotalOutput => Outputs.Sum(o => o.Amount);
    public bool IsBalanced => TotalInput == TotalOutput;
}

public enum IdentityStatus { Active, Revoked, Suspended }
public enum TransactionStatus { Pending, Confirmed, Failed }
```

**Key Principles**:
- Records = immutable value types
- Computed properties on domain models
- Validation logic encapsulated

### 3.2 Validation Rules (Enterprise Pattern)

```csharp
// File: Features/Domain/Validation/Rules.cs

public interface IValidationRule<T>
{
    ValidationResult Validate(T entity);
}

public class TransactionValidationRule : IValidationRule<Transaction>
{
    public ValidationResult Validate(Transaction tx)
    {
        var errors = new List<string>();
        
        if (!tx.IsBalanced)
            errors.Add("Transaction inputs must equal outputs");
        
        if (tx.Inputs.Count == 0)
            errors.Add("Transaction must have at least one input");
        
        if (tx.Outputs.Count == 0)
            errors.Add("Transaction must have at least one output");
        
        return new ValidationResult(errors.Count == 0, errors);
    }
}

public class TransactionValidator
{
    private readonly List<IValidationRule<Transaction>> _rules;
    
    public ValidationResult Validate(Transaction tx)
    {
        foreach (var rule in _rules)
        {
            var result = rule.Validate(tx);
            if (!result.IsValid)
                return result;
        }
        
        return ValidationResult.Success;
    }
}
```

**Separation**: Validation = separate rule objects, testable in isolation

---

## 4. Layer 3: Service Layer (Business Logic)

**Responsibility**: HTTP communication, transformations, complex operations

### 4.1 API Service (HTTP Client)

```csharp
// File: Features/Services/Api/ApiClient.cs

public interface IApiClient
{
    Task<IdentityResponse> RegisterIdentityAsync(IdentityRegistrationRequest req);
    Task<TransactionResponse> SubmitTransactionAsync(TransactionRequest req);
    Task<IdentityResponse> GetIdentityAsync(string did);
    Task<bool> VerifyIdentityAsync(string did, byte[] challenge);
}

public class RestApiClient : IApiClient
{
    private readonly HttpClient _httpClient;
    private readonly ILogger<RestApiClient> _logger;
    
    public RestApiClient(HttpClient httpClient, ILogger<RestApiClient> logger)
    {
        _httpClient = httpClient;
        _logger = logger;
    }
    
    public async Task<IdentityResponse> RegisterIdentityAsync(IdentityRegistrationRequest req)
    {
        try
        {
            var json = JsonSerializer.Serialize(req);
            var content = new StringContent(json, Encoding.UTF8, "application/json");
            
            var response = await _httpClient.PostAsync("/api/identity/register", content);
            response.EnsureSuccessStatusCode();
            
            var responseJson = await response.Content.ReadAsStringAsync();
            return JsonSerializer.Deserialize<IdentityResponse>(responseJson);
        }
        catch (HttpRequestException ex)
        {
            _logger.LogError($"Identity registration failed: {ex.Message}");
            throw new ServiceException("Registration failed", ex);
        }
    }
}
```

**Key Separation**:
- HTTP logic ≠ business logic
- Exceptions caught and logged here
- Errors transformed to domain exceptions

### 4.2 Domain Service Layer

```csharp
// File: Features/Services/Domain/IdentityService.cs

public interface IIdentityService
{
    Task<DigitalIdentity> RegisterAsync(string username, string email);
    Task<DigitalIdentity> GetAsync(string did);
    Task<bool> VerifyAsync(string did, byte[] challenge);
    Task<Credential> IssueCredentialAsync(string subjectDid, Dictionary<string, object> claims);
}

public class IdentityService : IIdentityService
{
    private readonly IApiClient _api;
    private readonly ILocalDatabase _db;
    private readonly IEncryptionService _encryption;
    private readonly ILogger<IdentityService> _logger;
    
    public IdentityService(
        IApiClient api,
        ILocalDatabase db,
        IEncryptionService encryption,
        ILogger<IdentityService> logger)
    {
        _api = api;
        _db = db;
        _encryption = encryption;
        _logger = logger;
    }
    
    public async Task<DigitalIdentity> RegisterAsync(string username, string email)
    {
        _logger.LogInformation($"Registering identity: {email}");
        
        // Step 1: Generate keypair locally
        var (publicKey, privateKey) = await _encryption.GenerateKeyPairAsync();
        
        // Step 2: Submit to backend
        var request = new IdentityRegistrationRequest
        {
            Username = username,
            Email = email,
            PublicKey = publicKey,
        };
        
        var response = await _api.RegisterIdentityAsync(request);
        
        // Step 3: Store locally (encrypted)
        var identity = new DigitalIdentity(
            response.Id,
            response.DID,
            publicKey,
            new List<Credential>(),
            IdentityStatus.Active,
            DateTime.UtcNow,
            DateTime.UtcNow
        );
        
        await _db.InsertAsync(identity);
        await _encryption.StorePrivateKeyAsync(identity.Id, privateKey);
        
        _logger.LogInformation($"Identity registered successfully: {response.DID}");
        return identity;
    }
}
```

**Key Responsibilities**:
- Orchestrate API + Local DB
- Transform API responses to domain models
- Handle business logic (keypair generation, etc.)
- Logging at every step

### 4.3 Specialized Services

```csharp
// File: Features/Services/Domain/TransactionService.cs

public interface ITransactionService
{
    Task<Transaction> CreateAsync(List<TransactionInput> inputs, List<TransactionOutput> outputs);
    Task<TransactionResponse> SubmitAsync(Transaction tx);
    Task<Transaction> GetAsync(string txId);
    Task<List<Transaction>> ListPendingAsync();
}

public class TransactionService : ITransactionService
{
    private readonly IApiClient _api;
    private readonly ILocalDatabase _db;
    private readonly IEncryptionService _encryption;
    private readonly ITransactionValidator _validator;
    
    public async Task<Transaction> CreateAsync(List<TransactionInput> inputs, List<TransactionOutput> outputs)
    {
        // Step 1: Create unsigned transaction
        var unsignedTx = new Transaction(
            Id: Guid.NewGuid().ToString(),
            Inputs: inputs,
            Outputs: outputs,
            Signature: "", // Not signed yet
            CreatedAt: DateTime.UtcNow,
            Status: TransactionStatus.Pending
        );
        
        // Step 2: Validate
        var validationResult = _validator.Validate(unsignedTx);
        if (!validationResult.IsValid)
            throw new DomainException($"Transaction validation failed: {string.Join(", ", validationResult.Errors)}");
        
        // Step 3: Sign locally
        var signature = await _encryption.SignAsync(unsignedTx);
        var signedTx = unsignedTx with { Signature = signature };
        
        // Step 4: Store locally
        await _db.InsertAsync(signedTx);
        
        return signedTx;
    }
    
    public async Task<TransactionResponse> SubmitAsync(Transaction tx)
    {
        var request = new TransactionRequest
        {
            Inputs = tx.Inputs,
            Outputs = tx.Outputs,
            Signature = tx.Signature,
        };
        
        var response = await _api.SubmitTransactionAsync(request);
        
        // Update local status
        var updatedTx = tx with { Status = TransactionStatus.Confirmed };
        await _db.UpdateAsync(updatedTx);
        
        return response;
    }
}
```

**Key Separation**:
- Creation ≠ Submission (different concerns)
- Validation separate from persistence
- Each method has single responsibility

---

## 5. Layer 4: ViewModel Layer (Presentation Logic)

**Responsibility**: UI state, user input handling, navigation

### 5.1 Base ViewModel

```csharp
// File: Features/UI/Base/BaseViewModel.cs

[INotifyPropertyChanged]
public partial class BaseViewModel
{
    protected readonly ILogger<BaseViewModel> Logger;
    protected readonly INavigationService NavigationService;
    
    [ObservableProperty]
    private bool isLoading;
    
    [ObservableProperty]
    private string errorMessage;
    
    [ObservableProperty]
    private bool hasError;
    
    public BaseViewModel(
        ILogger<BaseViewModel> logger,
        INavigationService navigationService)
    {
        Logger = logger;
        NavigationService = navigationService;
    }
    
    protected virtual Task OnInitializeAsync() => Task.CompletedTask;
    protected virtual Task OnAppearingAsync() => Task.CompletedTask;
    protected virtual Task OnDisappearingAsync() => Task.CompletedTask;
    
    protected void SetError(string message)
    {
        ErrorMessage = message;
        HasError = true;
        Logger.LogError(message);
    }
    
    protected void ClearError()
    {
        ErrorMessage = string.Empty;
        HasError = false;
    }
    
    protected async Task ExecuteAsync(Func<Task> action, string loadingMessage = "Loading...")
    {
        try
        {
            IsLoading = true;
            ClearError();
            await action();
        }
        catch (Exception ex)
        {
            SetError(ex.Message);
        }
        finally
        {
            IsLoading = false;
        }
    }
}
```

### 5.2 Identity ViewModel (Example)

```csharp
// File: Features/UI/Identity/IdentityViewModel.cs

[INotifyPropertyChanged]
public partial class IdentityViewModel : BaseViewModel
{
    private readonly IIdentityService _identityService;
    private readonly ISyncStateService _syncService;
    
    [ObservableProperty]
    private DigitalIdentity currentIdentity;
    
    [ObservableProperty]
    private List<Credential> credentials = new();
    
    [ObservableProperty]
    private string identityStatus;
    
    [RelayCommand]
    public async Task RegisterIdentityAsync(string username, string email)
    {
        await ExecuteAsync(async () =>
        {
            var identity = await _identityService.RegisterAsync(username, email);
            CurrentIdentity = identity;
            IdentityStatus = $"Identity registered: {identity.DID}";
        });
    }
    
    [RelayCommand]
    public async Task LoadIdentityAsync(string did)
    {
        await ExecuteAsync(async () =>
        {
            var identity = await _identityService.GetAsync(did);
            CurrentIdentity = identity;
            Credentials = identity.Credentials;
        });
    }
    
    [RelayCommand]
    public async Task IssueCredentialAsync(string subjectDid, Dictionary<string, object> claims)
    {
        await ExecuteAsync(async () =>
        {
            var credential = await _identityService.IssueCredentialAsync(subjectDid, claims);
            Credentials.Add(credential);
            await _syncService.MarkForSyncAsync(credential);
        });
    }
    
    protected override async Task OnInitializeAsync()
    {
        // Load cached identity on startup
        var cached = await _identityService.GetAsync(PreferencesService.CurrentDID);
        if (cached != null)
        {
            CurrentIdentity = cached;
            Credentials = cached.Credentials;
        }
    }
}
```

**Key Principles**:
- Observable properties only for UI state
- Commands for user interactions
- Services injected (never created)
- No HTTP calls directly
- Async/await with try-catch via ExecuteAsync

### 5.3 Transaction ViewModel (Example)

```csharp
// File: Features/UI/Transactions/TransactionViewModel.cs

[INotifyPropertyChanged]
public partial class TransactionViewModel : BaseViewModel
{
    private readonly ITransactionService _txService;
    private readonly ICryptoService _crypto;
    
    [ObservableProperty]
    private decimal inputAmount;
    
    [ObservableProperty]
    private string recipientAddress;
    
    [ObservableProperty]
    private List<Transaction> pendingTransactions = new();
    
    [RelayCommand]
    public async Task CreateTransactionAsync()
    {
        await ExecuteAsync(async () =>
        {
            // Create inputs
            var inputs = new List<TransactionInput>
            {
                new TransactionInput(InputAmount, CurrentIdentity.Id)
            };
            
            // Create outputs
            var outputs = new List<TransactionOutput>
            {
                new TransactionOutput(InputAmount, RecipientAddress)
            };
            
            // Create transaction (validates, signs locally)
            var tx = await _txService.CreateAsync(inputs, outputs);
            
            // Show confirmation
            await NavigationService.GoToAsync("transaction-confirm", new { transaction = tx });
        });
    }
    
    [RelayCommand]
    public async Task SubmitTransactionAsync(Transaction tx)
    {
        await ExecuteAsync(async () =>
        {
            var response = await _txService.SubmitAsync(tx);
            await NavigationService.GoToAsync("transaction-success", new { txId = response.Id });
        });
    }
    
    [RelayCommand]
    public async Task RefreshPendingAsync()
    {
        await ExecuteAsync(async () =>
        {
            PendingTransactions = await _txService.ListPendingAsync();
        });
    }
    
    protected override async Task OnAppearingAsync()
    {
        await RefreshPendingAsync();
    }
}
```

---

## 6. Layer 5: View Layer (XAML Only)

**Responsibility**: Layout only, zero code-behind

### 6.1 Identity View (Example)

```xml
<!-- File: Features/UI/Identity/IdentityPage.xaml -->

<base:BaseContentPage
    xmlns="http://schemas.microsoft.com/dotnet/2021/maui"
    xmlns:base="clr-namespace:NeuroAccess.UI.Pages"
    x:Class="NeuroAccess.Features.UI.Identity.IdentityPage"
    Title="Digital Identity">
    
    <VerticalStackLayout Padding="20" Spacing="15">
        
        <!-- Status Section -->
        <Frame BorderColor="Gray" CornerRadius="10" Padding="15">
            <VerticalStackLayout Spacing="10">
                <Label 
                    Text="Identity Status" 
                    FontSize="16" 
                    FontAttributes="Bold" />
                <Label 
                    Text="{Binding IdentityStatus}" 
                    FontSize="14" 
                    TextColor="Gray" />
            </VerticalStackLayout>
        </Frame>
        
        <!-- Register Button -->
        <Button
            Text="Register Identity"
            Command="{Binding RegisterIdentityCommand}"
            IsEnabled="{Binding !IsLoading}" />
        
        <!-- Loading Indicator -->
        <ActivityIndicator 
            IsRunning="{Binding IsLoading}"
            IsVisible="{Binding IsLoading}" />
        
        <!-- Error Display -->
        <Frame 
            IsVisible="{Binding HasError}"
            BorderColor="Red"
            CornerRadius="10"
            Padding="15">
            <Label 
                Text="{Binding ErrorMessage}" 
                TextColor="Red" />
        </Frame>
        
        <!-- Credentials List -->
        <CollectionView ItemsSource="{Binding Credentials}">
            <CollectionView.ItemTemplate>
                <DataTemplate>
                    <StackLayout Padding="10" Spacing="5">
                        <Label Text="{Binding Id, StringFormat='Credential: {0}'}" />
                        <Label Text="{Binding IssuedAt, StringFormat='Issued: {0:g}'}" />
                    </StackLayout>
                </DataTemplate>
            </CollectionView.ItemTemplate>
        </CollectionView>
        
    </VerticalStackLayout>
    
</base:BaseContentPage>
```

**Key Rules**:
- ZERO code-behind logic
- Data binding to ViewModel only
- Commands for all interactions
- Clean, readable XAML

### 6.2 View Code-Behind (Minimal)

```csharp
// File: Features/UI/Identity/IdentityPage.xaml.cs

public partial class IdentityPage : BaseContentPage
{
    public IdentityPage(IdentityViewModel viewModel)
    {
        InitializeComponent();
        BindingContext = viewModel;
    }
}
```

**Rules**:
- Constructor only for DI
- BindingContext assignment only
- ZERO other logic

---

## 7. Dependency Injection Setup

```csharp
// File: MauiProgram.cs

public static class MauiProgram
{
    public static MauiApp CreateMauiApp()
    {
        var builder = MauiApp.CreateBuilder();
        builder
            .UseMauiApp<App>()
            .ConfigureFonts(fonts => { /* ... */ })
            .ConfigureServices()
            .ConfigureViewsAndViewModels();
        
        return builder.Build();
    }
}

public static class ServiceCollectionExtensions
{
    public static MauiAppBuilder ConfigureServices(this MauiAppBuilder builder)
    {
        builder.Services.AddSingleton<IEncryptionService, EncryptionService>();
        builder.Services.AddSingleton<ILocalDatabase>(sp =>
            new EncryptedSqliteDatabase(
                GetDbPath(),
                sp.GetRequiredService<IEncryptionService>()
            )
        );
        
        builder.Services.AddSingleton<IApiClient>(sp =>
            new RestApiClient(
                CreateHttpClient(),
                sp.GetRequiredService<ILogger<RestApiClient>>()
            )
        );
        
        // Domain Services
        builder.Services.AddSingleton<IIdentityService, IdentityService>();
        builder.Services.AddSingleton<ITransactionService, TransactionService>();
        builder.Services.AddSingleton<ISyncStateService, SyncStateService>();
        
        // Navigation
        builder.Services.AddSingleton<INavigationService, NavigationService>();
        
        return builder;
    }
    
    public static MauiAppBuilder ConfigureViewsAndViewModels(this MauiAppBuilder builder)
    {
        // Pages
        builder.Services.AddTransient<IdentityPage>();
        builder.Services.AddTransient<TransactionPage>();
        
        // ViewModels
        builder.Services.AddTransient<IdentityViewModel>();
        builder.Services.AddTransient<TransactionViewModel>();
        
        return builder;
    }
}
```

---

## 8. Testing Strategy

### 8.1 ViewModel Unit Tests

```csharp
[TestClass]
public class IdentityViewModelTests
{
    private Mock<IIdentityService> _mockIdentityService;
    private Mock<INavigationService> _mockNavigationService;
    private Mock<ILogger<IdentityViewModel>> _mockLogger;
    private IdentityViewModel _viewModel;
    
    [TestInitialize]
    public void Setup()
    {
        _mockIdentityService = new Mock<IIdentityService>();
        _mockNavigationService = new Mock<INavigationService>();
        _mockLogger = new Mock<ILogger<IdentityViewModel>>();
        
        _viewModel = new IdentityViewModel(
            _mockIdentityService.Object,
            _mockNavigationService.Object,
            _mockLogger.Object
        );
    }
    
    [TestMethod]
    public async Task RegisterIdentityAsync_Success_UpdatesCurrentIdentity()
    {
        // Arrange
        var testIdentity = new DigitalIdentity(
            Id: "id-123",
            DID: "did:neuro:test",
            PublicKey: "key",
            Credentials: new(),
            Status: IdentityStatus.Active,
            CreatedAt: DateTime.UtcNow,
            UpdatedAt: DateTime.UtcNow
        );
        
        _mockIdentityService
            .Setup(s => s.RegisterAsync(It.IsAny<string>(), It.IsAny<string>()))
            .ReturnsAsync(testIdentity);
        
        // Act
        await _viewModel.RegisterIdentityCommand.ExecuteAsync(null);
        
        // Assert
        Assert.AreEqual(testIdentity, _viewModel.CurrentIdentity);
        Assert.IsFalse(_viewModel.IsLoading);
        Assert.IsFalse(_viewModel.HasError);
    }
    
    [TestMethod]
    public async Task RegisterIdentityAsync_Failure_SetsError()
    {
        // Arrange
        _mockIdentityService
            .Setup(s => s.RegisterAsync(It.IsAny<string>(), It.IsAny<string>()))
            .ThrowsAsync(new Exception("Registration failed"));
        
        // Act
        await _viewModel.RegisterIdentityCommand.ExecuteAsync(null);
        
        // Assert
        Assert.IsTrue(_viewModel.HasError);
        Assert.AreEqual("Registration failed", _viewModel.ErrorMessage);
    }
}
```

### 8.2 Service Unit Tests

```csharp
[TestClass]
public class IdentityServiceTests
{
    private Mock<IApiClient> _mockApi;
    private Mock<ILocalDatabase> _mockDb;
    private Mock<IEncryptionService> _mockEncryption;
    private IdentityService _service;
    
    [TestInitialize]
    public void Setup()
    {
        _mockApi = new Mock<IApiClient>();
        _mockDb = new Mock<ILocalDatabase>();
        _mockEncryption = new Mock<IEncryptionService>();
        
        _service = new IdentityService(_mockApi.Object, _mockDb.Object, _mockEncryption.Object, null);
    }
    
    [TestMethod]
    public async Task RegisterAsync_Success_StoresLocallyAndReturnsIdentity()
    {
        // Arrange
        var response = new IdentityResponse { Id = "id-1", DID = "did:neuro:1" };
        _mockApi.Setup(a => a.RegisterIdentityAsync(It.IsAny<IdentityRegistrationRequest>()))
            .ReturnsAsync(response);
        _mockEncryption.Setup(e => e.GenerateKeyPairAsync())
            .ReturnsAsync(("pub-key", "priv-key"));
        
        // Act
        var result = await _service.RegisterAsync("user", "user@example.com");
        
        // Assert
        Assert.AreEqual(response.DID, result.DID);
        _mockDb.Verify(d => d.InsertAsync(It.IsAny<DigitalIdentity>()), Times.Once);
    }
}
```

---

## 9. Summary: Layer Responsibilities

| Layer | Responsibility | Can Call | Cannot Call |
|-------|---|---|---|
| **View** | Layout only | ViewModel commands | Services, API |
| **ViewModel** | UI state, orchestration | Services, Navigation | API directly |
| **Service** | Business logic, HTTP | API, DB, Validation | ViewModel, View |
| **Model** | Data contracts | Validation rules | Services |
| **Persistence** | Local storage | Encryption | HTTP, Business logic |

**Principle**: Unidirectional dependency flow → No circular dependencies

---

## 10. Offline-First Architecture

### Sync Queue Pattern

```csharp
// File: Features/Services/Sync/SyncQueue.cs

public class SyncQueue
{
    private readonly IApiClient _api;
    private readonly ISyncStateService _syncState;
    private readonly Channel<SyncItem> _queue;
    
    public async Task EnqueueAsync<T>(T entity) where T : class
    {
        var item = new SyncItem(entity.GetType().Name, JsonSerializer.Serialize(entity));
        await _queue.Writer.WriteAsync(item);
        await _syncState.MarkForSyncAsync(entity);
    }
    
    public async Task ProcessQueueAsync()
    {
        while (await _queue.Reader.WaitToReadAsync())
        {
            while (_queue.Reader.TryRead(out var item))
            {
                try
                {
                    await _api.SyncAsync(item);
                    await _syncState.MarkSyncedAsync(item);
                }
                catch (Exception ex)
                {
                    await _syncState.MarkErrorAsync(item, ex.Message);
                }
            }
        }
    }
}
```

**Key**: Queue persisted to local DB → No data loss on app crash

---

**End of Frontend Architecture Design**

*Next: Task 3 - Integration Layer (REST bridge between backend and frontend)*
