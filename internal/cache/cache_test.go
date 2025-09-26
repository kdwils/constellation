package cache_test

import (
	"testing"

	"github.com/kdwils/constellation/internal/cache"
)

func TestCache_Set(t *testing.T) {
	tests := []struct {
		name  string
		key   string
		value string
	}{
		{
			name:  "set value",
			key:   "test-key",
			value: "test-value",
		},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			c := cache.New[string]()
			c.Set(tt.key, tt.value)
			gotValue, ok := c.Get(tt.key)
			if !ok {
				t.Errorf("TestCache_Set() key %s does not exist", tt.key)
			}
			if gotValue != tt.value {
				t.Errorf("TestCache_Set() unexpected value %v", gotValue)
			}
		})
	}
}

func TestCache_Get(t *testing.T) {
	tests := []struct {
		name     string
		key      string
		setValue string
		wantOk   bool
	}{
		{
			name:     "get existing value",
			key:      "test-key",
			setValue: "test-value",
			wantOk:   true,
		},
		{
			name:   "get non-existing value",
			key:    "missing-key",
			wantOk: false,
		},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			c := cache.New[string]()
			if tt.wantOk {
				c.Set(tt.key, tt.setValue)
			}
			gotValue, ok := c.Get(tt.key)
			if ok != tt.wantOk {
				t.Errorf("TestCache_Get() ok = %v, want %v", ok, tt.wantOk)
			}
			if tt.wantOk && gotValue != tt.setValue {
				t.Errorf("TestCache_Get() value = %v, want %v", gotValue, tt.setValue)
			}
		})
	}
}

func TestCache_Delete(t *testing.T) {
	tests := []struct {
		name      string
		key       string
		setValue  string
		deleteKey string
	}{
		{
			name:      "delete existing value",
			key:       "test-key",
			setValue:  "test-value",
			deleteKey: "test-key",
		},
		{
			name:      "delete non-existing value",
			key:       "test-key",
			setValue:  "test-value",
			deleteKey: "missing-key",
		},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			c := cache.New[string]()
			c.Set(tt.key, tt.setValue)
			c.Delete(tt.deleteKey)
			_, ok := c.Get(tt.deleteKey)
			if ok && tt.key == tt.deleteKey {
				t.Errorf("TestCache_Delete() key %s should not exist after deletion", tt.deleteKey)
			}
		})
	}
}

func TestCache_Size(t *testing.T) {
	tests := []struct {
		name     string
		entries  map[string]string
		wantSize int
	}{
		{
			name:     "empty cache",
			entries:  map[string]string{},
			wantSize: 0,
		},
		{
			name: "cache with entries",
			entries: map[string]string{
				"key1": "value1",
				"key2": "value2",
				"key3": "value3",
			},
			wantSize: 3,
		},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			c := cache.New[string]()
			for key, value := range tt.entries {
				c.Set(key, value)
			}
			gotSize := c.Size()
			if gotSize != tt.wantSize {
				t.Errorf("TestCache_Size() size = %v, want %v", gotSize, tt.wantSize)
			}
		})
	}
}

func TestCache_Keys(t *testing.T) {
	tests := []struct {
		name    string
		entries map[string]string
	}{
		{
			name:    "empty cache",
			entries: map[string]string{},
		},
		{
			name: "cache with entries",
			entries: map[string]string{
				"key1": "value1",
				"key2": "value2",
				"key3": "value3",
			},
		},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			c := cache.New[string]()
			for key, value := range tt.entries {
				c.Set(key, value)
			}
			gotKeys := c.Keys()
			if len(gotKeys) != len(tt.entries) {
				t.Errorf("TestCache_Keys() keys length = %v, want %v", len(gotKeys), len(tt.entries))
			}
			keyMap := make(map[string]bool)
			for _, key := range gotKeys {
				keyMap[key] = true
			}
			for expectedKey := range tt.entries {
				if !keyMap[expectedKey] {
					t.Errorf("TestCache_Keys() missing key %s", expectedKey)
				}
			}
		})
	}
}

func TestCache_List(t *testing.T) {
	tests := []struct {
		name    string
		entries map[string]string
	}{
		{
			name:    "empty cache",
			entries: map[string]string{},
		},
		{
			name: "cache with entries",
			entries: map[string]string{
				"key1": "value1",
				"key2": "value2",
				"key3": "value3",
			},
		},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			c := cache.New[string]()
			for key, value := range tt.entries {
				c.Set(key, value)
			}
			gotValues := c.List()
			if len(gotValues) != len(tt.entries) {
				t.Errorf("TestCache_List() values length = %v, want %v", len(gotValues), len(tt.entries))
			}
			valueMap := make(map[string]bool)
			for _, value := range gotValues {
				valueMap[value] = true
			}
			for _, expectedValue := range tt.entries {
				if !valueMap[expectedValue] {
					t.Errorf("TestCache_List() missing value %s", expectedValue)
				}
			}
		})
	}
}
