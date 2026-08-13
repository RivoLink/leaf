# Responsive Mermaid rendering

At narrow widths, Leaf reflows oversized horizontal flowcharts and uses stacked class cards when
the original two-dimensional class layout cannot fit without wrapping.

## Horizontal flowchart

```mermaid
flowchart LR
    File[Path]
    Service[SourceService.upload_file]
    Capability{SOURCE_UPLOAD supported?}
    Workflow[Selected backend owns<br/>register + transfer + reconcile]
    Receipt[Phase-aware private<br/>upload receipt]
    Stream[Backend-specific Scotty<br/>start + upload/finalize]
    Reconcile[Same-backend source read]
    Result[Source or outcome-unknown<br/>error]

    File --> Service --> Capability
    Capability -->|no| Unsupported[UnsupportedOperationError<br/>before I/O]
    Capability -->|yes| Workflow --> Receipt --> Stream --> Reconcile --> Result
```

## Class diagram

```mermaid
classDiagram
    class Client {
        +BackendCapabilities capabilities
        +StudioCatalogService studio
        +AudioOverviewService audio_overviews
        +from_profile()
        +close()
    }
    class BackendAdapter {
        <<Protocol>>
        +invoke(OperationDef, input, deadline)
        +stream(OperationDef, input, deadline)
        +close()
    }
    class MobileBackend {
        -GrpcTransport transport
        -MobileCodec codec
        +invoke()
        +stream()
    }
    class WebBackend {
        -RpcTransport transport
        -WebCodec codec
        +invoke()
        +stream()
    }
    class CatalogService {
        -backend
        +list()
        +get()
    }
    class QuizService {
        -backend
        +create_quiz()
        +wait_quiz()
    }
    class ResearchService {
        -backend
        +start_deep()
        +import_results()
    }
    class ArtifactSummary {
        +artifact_id
        +kind
        +status
    }

    Client *-- BackendAdapter
    Client *-- CatalogService
    Client *-- QuizService
    Client *-- ResearchService
    BackendAdapter <|.. MobileBackend
    BackendAdapter <|.. WebBackend
    CatalogService --> BackendAdapter
    CatalogService --> ArtifactSummary
    QuizService --> BackendAdapter
    QuizService --> ArtifactSummary
    ResearchService --> BackendAdapter
```
