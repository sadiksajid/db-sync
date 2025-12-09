# Testing Database Objects Migration

## Step 1: Create Test Objects in MySQL

Run the test SQL script to create views, functions, procedures, and triggers in your MySQL database:

```bash
mysql -h 192.168.1.237 -u root -p testing < test_database_objects.sql
```

Or connect to MySQL and run it interactively:

```bash
mysql -h 192.168.1.237 -u root -p testing

mysql> source test_database_objects.sql
```

### What Gets Created

The script creates:

- **5 Views**:
  - `customer_summary` - Customer statistics with order counts
  - `active_customers` - Filtered view of active customers
  - `product_inventory` - Products with calculated discounted prices
  - `order_details` - Complex view with multiple joins
  - `low_stock_products` - Products with low inventory

- **5 Functions**:
  - `calculate_discount(price, discount_pct)` - Calculate discounted price
  - `days_since_order(order_date)` - Calculate days since order
  - `get_customer_status(customer_id)` - Get customer status
  - `calculate_order_total(order_id)` - Calculate order total
  - `check_stock_availability(product_id, qty)` - Check if stock is available

- **5 Procedures**:
  - `update_customer_status(customer_id, status)` - Update customer status
  - `add_order(customer_id, total, OUT order_id)` - Create new order
  - `update_product_stock(product_id, qty_change)` - Update stock quantity
  - `get_customer_orders(customer_id)` - Retrieve customer orders
  - `calculate_customer_ltv(customer_id, OUT lifetime_value)` - Calculate lifetime value

- **6 Triggers**:
  - `before_order_insert` - Set timestamps before inserting orders
  - `after_order_insert` - Log order insertions to audit table
  - `before_product_update` - Validate stock before updates
  - `after_product_update` - Log price changes
  - `after_customer_delete` - Log customer deletions
  - `before_customer_update` - Auto-update timestamp

## Step 2: Verify Objects in MySQL

Check what was created:

```sql
-- Show views
SELECT TABLE_NAME FROM INFORMATION_SCHEMA.VIEWS 
WHERE TABLE_SCHEMA = 'testing';

-- Show functions
SELECT ROUTINE_NAME FROM INFORMATION_SCHEMA.ROUTINES
WHERE ROUTINE_SCHEMA = 'testing' AND ROUTINE_TYPE = 'FUNCTION';

-- Show procedures
SELECT ROUTINE_NAME FROM INFORMATION_SCHEMA.ROUTINES
WHERE ROUTINE_SCHEMA = 'testing' AND ROUTINE_TYPE = 'PROCEDURE';

-- Show triggers
SELECT TRIGGER_NAME, EVENT_OBJECT_TABLE FROM INFORMATION_SCHEMA.TRIGGERS
WHERE TRIGGER_SCHEMA = 'testing';
```

## Step 3: Run the Migration

Run the full sync to migrate everything to PostgreSQL:

```bash
./rebuild-and-run.sh --full-sync
```

Watch the output for database objects migration:

```
INFO  Reading database objects from MySQL...
INFO  Found 5 views, 5 functions, 5 procedures, 6 triggers
INFO  Migrating database objects to PostgreSQL...
INFO  === Starting database objects migration ===

INFO  Migrating 5 views to PostgreSQL...
INFO  ✓ Created view: customer_summary
INFO  ✓ Created view: active_customers
INFO  ✓ Created view: product_inventory
INFO  ✓ Created view: order_details
INFO  ✓ Created view: low_stock_products
INFO  View migration complete: 5 successful, 0 failed

INFO  Migrating 5 functions to PostgreSQL...
INFO  ✓ Created function: calculate_discount
INFO  ✓ Created function: days_since_order
...
INFO  Function migration complete: 5 successful, 0 failed

INFO  Migrating 5 procedures to PostgreSQL...
...
INFO  Procedure migration complete: 5 successful, 0 failed

INFO  Migrating 6 triggers to PostgreSQL...
...
INFO  Trigger migration complete: 6 successful, 0 failed

INFO  === Database objects migration complete ===
```

## Step 4: Verify Objects in PostgreSQL

Connect to PostgreSQL and verify:

```bash
psql -h 192.168.1.237 -U postgres -d testing
```

### Check Views

```sql
-- List all views
SELECT table_name 
FROM information_schema.views 
WHERE table_schema = 'public'
ORDER BY table_name;

-- Test a view
SELECT * FROM customer_summary LIMIT 5;
```

### Check Functions

```sql
-- List all functions
SELECT routine_name, routine_type 
FROM information_schema.routines 
WHERE routine_schema = 'public'
ORDER BY routine_name;

-- Test a function
SELECT calculate_discount(100.00, 10);
SELECT days_since_order('2024-01-01'::timestamp);
```

### Check Procedures (converted to functions)

```sql
-- Procedures are converted to functions returning void
SELECT routine_name 
FROM information_schema.routines 
WHERE routine_schema = 'public' 
AND data_type = 'void'
ORDER BY routine_name;

-- Test a procedure (now a function)
SELECT update_customer_status(1, 'active');
```

### Check Triggers

```sql
-- List all triggers
SELECT trigger_name, event_object_table, action_timing, event_manipulation
FROM information_schema.triggers
WHERE trigger_schema = 'public'
ORDER BY event_object_table, trigger_name;

-- Test a trigger by inserting data
INSERT INTO orders (customer_id, total_amount, status) 
VALUES (1, 99.99, 'pending');

-- Check if trigger fired (check audit log)
SELECT * FROM audit_log ORDER BY changed_at DESC LIMIT 5;
```

## Step 5: Test Real-Time Sync with Database Objects

After migration, test that changes to objects are handled:

1. **In MySQL**, modify a view:
```sql
CREATE OR REPLACE VIEW active_customers AS
SELECT 
    customer_id,
    name,
    email,
    total_spent,
    created_at
FROM customers
WHERE status = 'active';
```

2. **In MySQL**, create a new function:
```sql
DELIMITER $$
CREATE FUNCTION test_new_function(x INT)
RETURNS INT
DETERMINISTIC
BEGIN
    RETURN x * 2;
END$$
DELIMITER ;
```

3. Run another migration to pick up changes:
```bash
./rebuild-and-run.sh --initial-sync
```

## Troubleshooting

### If a function fails to migrate

Check the logs for the error:

```
ERROR ✗ Failed to create function calculate_complex: syntax error
WARN   SQL was: CREATE OR REPLACE FUNCTION...
WARN   Note: Function conversion may require manual adjustment
```

Manually create it in PostgreSQL with corrected syntax.

### If a trigger fails

Triggers are more complex. Check PostgreSQL logs:

```bash
docker logs <postgres-container>
```

Or in `psql`:

```sql
-- Check for errors
SHOW log_destination;
```

### Common Issues

1. **MySQL IF() function**: Convert to CASE expression
   ```sql
   -- MySQL
   SELECT IF(status='active', 'yes', 'no') FROM customers;
   
   -- PostgreSQL
   SELECT CASE WHEN status='active' THEN 'yes' ELSE 'no' END FROM customers;
   ```

2. **GROUP_CONCAT()**: Convert to STRING_AGG()
   ```sql
   -- MySQL
   SELECT GROUP_CONCAT(name) FROM customers;
   
   -- PostgreSQL
   SELECT STRING_AGG(name, ',') FROM customers;
   ```

3. **DATEDIFF()**: Use date subtraction
   ```sql
   -- MySQL
   SELECT DATEDIFF(NOW(), order_date) FROM orders;
   
   -- PostgreSQL
   SELECT CURRENT_DATE - order_date::date FROM orders;
   ```

## Cleanup

To remove test objects from MySQL:

```sql
-- Drop views
DROP VIEW IF EXISTS customer_summary;
DROP VIEW IF EXISTS active_customers;
DROP VIEW IF EXISTS product_inventory;
DROP VIEW IF EXISTS order_details;
DROP VIEW IF EXISTS low_stock_products;

-- Drop functions
DROP FUNCTION IF EXISTS calculate_discount;
DROP FUNCTION IF EXISTS days_since_order;
DROP FUNCTION IF EXISTS get_customer_status;
DROP FUNCTION IF EXISTS calculate_order_total;
DROP FUNCTION IF EXISTS check_stock_availability;

-- Drop procedures
DROP PROCEDURE IF EXISTS update_customer_status;
DROP PROCEDURE IF EXISTS add_order;
DROP PROCEDURE IF EXISTS update_product_stock;
DROP PROCEDURE IF EXISTS get_customer_orders;
DROP PROCEDURE IF EXISTS calculate_customer_ltv;

-- Drop triggers
DROP TRIGGER IF EXISTS before_order_insert;
DROP TRIGGER IF EXISTS after_order_insert;
DROP TRIGGER IF EXISTS before_product_update;
DROP TRIGGER IF EXISTS after_product_update;
DROP TRIGGER IF EXISTS after_customer_delete;
DROP TRIGGER IF EXISTS before_customer_update;
```

To remove from PostgreSQL:

```sql
-- Drop views
DROP VIEW IF EXISTS customer_summary CASCADE;
DROP VIEW IF EXISTS active_customers CASCADE;
-- ... etc

-- Drop functions (including converted procedures)
DROP FUNCTION IF EXISTS calculate_discount(NUMERIC, INTEGER) CASCADE;
-- ... etc

-- Drop triggers (drop the trigger and its function)
DROP TRIGGER IF EXISTS before_order_insert ON orders;
DROP FUNCTION IF EXISTS before_order_insert_func() CASCADE;
-- ... etc
```

## Success Criteria

✅ All 5 views should be visible in PostgreSQL
✅ All 5 functions should work in PostgreSQL  
✅ All 5 procedures (as functions) should exist in PostgreSQL
✅ All 6 triggers should fire correctly in PostgreSQL
✅ Views return the same data in both databases
✅ Functions return the same results in both databases
✅ Triggers log correctly to audit_log in both databases

Happy testing! 🚀

