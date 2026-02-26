# rarch (归藏) Development Plan

This document outlines the strategic roadmap for the development of **rarch**. Each phase represents a significant milestone in transitioning the tool from a robust utility to a comprehensive data management ecosystem.

## Phase 1: Foundation & Reliability
*Target: Technical maturity and core stability.*

- **Enhanced Journaling**: Implementation of redundant journal backups to ensure 100% recovery even in the event of hardware failure during operations.
- **Extended Content Detection**: Expansion of the `infer` engine to support specialized scientific, engineering, and rare media formats.
- **TUI Refinement**: Introduction of a configurable dashboard with real-time performance metrics and interactive rule auditing.
- **Comprehensive Test Suite**: Incorporation of property-based testing for high-risk operations like hard-link deduplication.

## Phase 2: Performance Optimization
*Target: Reaching the limits of modern storage hardware.*

- **Zero-Copy I/O**: Utilization of `sendfile` and `splice` (on Linux) to minimize CPU overhead during file movement across the same filesystem.
- **Asynchronous Engine**: Migration of the scanning and processing pipeline to a fully asynchronous model to maximize IOPS utilization on NVMe storage.
- **Deterministic Deduplication**: Optimization of the SHA-256 pipeline using SIMD instructions for faster large-scale file hashing.
- **Memory Profiling**: Drastic reduction of the memory footprint during "Watch Mode" for background operation on low-power devices.

## Phase 3: Automation & Integration
*Target: From a tool to a service.*

- **Scripting Engine**: Integration of a lightweight scripting language (e.g., Rhai) to allow users to define complex, stateful organization logic.
- **Cloud Gateway**: Experimental support for organizing files on remote storage via Rclone or native S3/WebDAV integrations.
- **Notification System**: Webhook and desktop notification support for reporting batch operation summaries and security alerts.
- **Advanced Triggers**: Condition-based execution (e.g., "Run cleanup only when disk usage exceeds 80%").

## Phase 4: Ecosystem & Expansion
*Target: Broad accessibility and community growth.*

- **rarch Registry**: A centralized platform for the community to share and discover organization rule templates for various workflows (Photography, Development, Archiving).
- **Desktop Graphical Interface**: Development of a cross-platform GUI (powered by Tauri) for users who prefer visual interaction over Terminal environments.
- **Mobile Companion**: A simplified version of rarch for mobile devices to help organize local media and assets.
- **API for Developers**: Formalizing the core engine as a library (`librarch`) to enable other developers to build content-aware file tools.
