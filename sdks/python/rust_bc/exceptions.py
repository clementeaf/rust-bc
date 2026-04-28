"""Custom exceptions for the rust-bc SDK."""


class BlockchainError(Exception):
    """Base exception for all SDK errors."""

    def __init__(self, message: str, status_code: int | None = None) -> None:
        super().__init__(message)
        self.status_code = status_code


class ConnectionError(BlockchainError):
    """Failed to connect to the blockchain node."""


class NotFoundError(BlockchainError):
    """Requested resource was not found (404)."""


class ForbiddenError(BlockchainError):
    """Access denied — missing identity or insufficient permissions (403)."""


class ValidationError(BlockchainError):
    """Request validation failed (400)."""
