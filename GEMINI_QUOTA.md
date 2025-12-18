# 🔄 Gemini API Quota Handling

## Problem

Gemini API free tier has strict quotas:
- **`gemini-2.5-flash`**: 20 requests/day (⚠️ **VERY LIMITED**)
- **`gemini-2.0-flash-exp`**: More generous limits

When you exceed the quota, you get `429 Too Many Requests` errors, and database objects fail to migrate.

## Solution

The proxy now **automatically handles quota exceeded errors** with intelligent fallback:

### ✅ Smart Fallback Strategy

```
1. Try Gemini AI conversion
   ↓ (if 429 quota error)
2. Automatically fall back to regex-based conversion
   ↓
3. Continue migrating remaining objects
```

### ✅ No Manual Intervention Required

The system handles quota exhaustion automatically - you don't need to:
- Restart the migration
- Wait 24 hours for quota reset
- Manually convert failed objects

### ✅ Best Effort Migration

- **Gemini-converted objects**: High accuracy, handles complex SQL
- **Regex-converted objects**: Good for simple objects, may need manual review
- **Result**: Most objects migrate successfully, even after hitting quota

## Recommendations

### 1. Use `gemini-2.0-flash-exp` (Free, More Generous)

```bash
# In rebuild-and-run.sh (default)
export GEMINI_MODEL="gemini-2.0-flash-exp"
```

✅ **Recommended for most users**
- Much higher quota
- Still free
- Good accuracy

### 2. Or use `gemini-2.5-flash` (Limited)

```bash
export GEMINI_MODEL="gemini-2.5-flash"
```

⚠️ **Only 20 requests/day**
- Use for small databases (< 20 objects)
- Best accuracy
- Hits quota quickly

### 3. Monitor Your Usage

Check your quota at: https://ai.dev/usage?tab=rate-limit

## What Happens When Quota is Exceeded

### Before (Old Behavior) ❌
```
Error: Gemini API quota exceeded
Migration stops
Database objects not migrated
Manual intervention required
```

### After (New Behavior) ✅
```
Warning: Gemini quota exceeded for view: customer_summary
Falling back to regex converter...
✓ Created view (using regex): customer_summary
Migration continues automatically
```

## Console Output Example

```
📊 Migrating 5 views to PostgreSQL...
2025-12-12T14:00:48.683560Z  INFO Using Gemini AI to convert view: active_customers
2025-12-12T14:01:48.683560Z  INFO ✓ Created view: active_customers

2025-12-12T14:02:48.683560Z  INFO Using Gemini AI to convert view: customer_summary
2025-12-12T14:03:48.683560Z  WARN ⚠️ Gemini API quota exceeded: 429 Too Many Requests
2025-12-12T14:03:48.683562Z  WARN ⚠️ Gemini quota exceeded, falling back to regex converter for view: customer_summary
2025-12-12T14:03:48.683565Z  INFO ✓ Created view (using regex): customer_summary

2025-12-12T14:04:48.683560Z  WARN ⚠️ Gemini quota exceeded, falling back to regex converter for view: low_stock_products
2025-12-12T14:04:48.683565Z  INFO ✓ Created view (using regex): low_stock_products

...

View migration complete: 1 successful (Gemini), 4 successful (regex), 0 failed
```

## Quota Reset

- **Free tier**: Quota resets daily
- **Check reset time**: https://ai.dev/usage?tab=rate-limit
- **Strategy**: If you have many objects, run migration after quota reset

## Best Practices

### For Small Databases (< 20 objects)

```bash
# Use gemini-2.5-flash for best accuracy
export GEMINI_MODEL="gemini-2.5-flash"
./rebuild-and-run.sh --full-sync
```

✅ All objects will use Gemini AI
✅ Highest accuracy
✅ Won't hit quota

### For Medium Databases (20-100 objects)

```bash
# Use gemini-2.0-flash-exp (higher quota)
export GEMINI_MODEL="gemini-2.0-flash-exp"
./rebuild-and-run.sh --full-sync
```

✅ Most objects will use Gemini AI
✅ Automatic fallback for remaining objects
✅ Good balance

### For Large Databases (> 100 objects)

**Option 1**: Split migration over multiple days
```bash
# Day 1: Migrate first batch
export GEMINI_MODEL="gemini-2.0-flash-exp"
./rebuild-and-run.sh --initial-sync

# Day 2: After quota reset, manually migrate failed objects
```

**Option 2**: Use regex only (no Gemini)
```bash
# Skip Gemini completely
unset GEMINI_API_KEY
./rebuild-and-run.sh --full-sync
```

✅ No quota issues
⚠️ May need manual review for complex objects

## Troubleshooting

### Issue: Too many objects failing

**Solution**: You hit quota early in the migration

```bash
# Check how many objects you have
mysql> SELECT COUNT(*) FROM information_schema.views WHERE table_schema = 'your_db';
mysql> SELECT COUNT(*) FROM information_schema.routines WHERE routine_schema = 'your_db';
mysql> SELECT COUNT(*) FROM information_schema.triggers WHERE trigger_schema = 'your_db';
```

If total > 20, use `gemini-2.0-flash-exp` or spread migration over days.

### Issue: Regex conversion failed

**Example**:
```
✗ Failed to convert view product_summary (regex fallback also failed): syntax error
```

**Solution**: Manual migration for complex objects
```sql
-- In MySQL: Get the definition
SHOW CREATE VIEW product_summary;

-- Manually convert to PostgreSQL syntax
-- Create in PostgreSQL manually
```

### Issue: Want to verify regex conversions

**Recommendation**: After migration, test the converted objects

```sql
-- Test views
SELECT * FROM customer_summary LIMIT 10;

-- Test functions
SELECT calculate_discount(100, 10);

-- Test procedures
CALL update_product_stock(1, 50);
```

## Configuration Reference

### Model Comparison

| Model | Quota | Accuracy | Speed | Recommendation |
|-------|-------|----------|-------|----------------|
| `gemini-2.5-flash` | 20/day | ⭐⭐⭐⭐⭐ | Fast | Small DBs only |
| `gemini-2.0-flash-exp` | Higher | ⭐⭐⭐⭐ | Fast | **Default** |
| Regex (fallback) | Unlimited | ⭐⭐⭐ | Instant | Automatic |

### Environment Variables

```bash
# Gemini API key (get from https://makersuite.google.com/app/apikey)
export GEMINI_API_KEY="YOUR_API_KEY"

# Gemini model (choose one)
export GEMINI_MODEL="gemini-2.0-flash-exp"  # Recommended
# OR
export GEMINI_MODEL="gemini-2.5-flash"      # Limited quota

# To skip Gemini completely
unset GEMINI_API_KEY
```

## Summary

✅ **Automatic fallback** when quota exceeded  
✅ **No manual intervention** required  
✅ **Best effort migration** continues  
✅ **Use `gemini-2.0-flash-exp`** for most cases  
✅ **Monitor usage** at ai.dev/usage  

---

**The migration will complete successfully even if you hit quota limits!** 🎉

