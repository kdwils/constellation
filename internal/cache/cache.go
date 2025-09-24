package cache

import (
	"sync"
)

// Cache provides a thread-safe generic cache implementation
type Cache[T any] struct {
	entries map[string]T
	mu      sync.RWMutex
}

// New creates a new cache
func New[T any]() *Cache[T] {
	return &Cache[T]{
		entries: make(map[string]T),
	}
}

// Set adds or updates an entry in the cache
func (c *Cache[T]) Set(key string, value T) {
	c.mu.Lock()
	defer c.mu.Unlock()
	c.entries[key] = value
}

// Get retrieves an entry from the cache
func (c *Cache[T]) Get(key string) (T, bool) {
	c.mu.RLock()
	defer c.mu.RUnlock()
	value, exists := c.entries[key]
	return value, exists
}

// Delete removes an entry from the cache
func (c *Cache[T]) Delete(key string) {
	c.mu.Lock()
	defer c.mu.Unlock()
	delete(c.entries, key)
}

// Size returns the number of entries in the cache
func (c *Cache[T]) Size() int {
	c.mu.RLock()
	defer c.mu.RUnlock()
	return len(c.entries)
}

// Keys returns all keys in the cache
func (c *Cache[T]) Keys() []string {
	c.mu.RLock()
	defer c.mu.RUnlock()
	
	keys := make([]string, 0, len(c.entries))
	for key := range c.entries {
		keys = append(keys, key)
	}
	return keys
}

// List returns all values in the cache
func (c *Cache[T]) List() []T {
	c.mu.RLock()
	defer c.mu.RUnlock()
	
	values := make([]T, 0, len(c.entries))
	for _, value := range c.entries {
		values = append(values, value)
	}
	return values
}