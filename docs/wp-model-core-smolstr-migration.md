# wp-model-core SmolStr 迁移状态

## 检查时间
2026-01-02

## 迁移状态总结

### ✅ 已迁移到 SmolStr (FNameStr)
- **Field.name** - 字段名称
- **所有 Maker API** - from_bool, from_chars, from_digit 等所有函数的 name 参数

### ❌ 仍使用 ArcStr
- **Value.Chars** - 字符串值
- **Value.Symbol** - 符号值
- **ObjectValue key** - 对象的键（使用 BTreeMap<ArcStr, DataField>）
- **IdCardT** - 身份证号
- **MobilePhoneT** - 手机号

## 设计决策分析

这是**合理的混合策略**：

### 为什么字段名用 SmolStr？
```rust
pub struct Field<T> {
    pub meta: DataType,
    pub name: FNameStr,  // SmolStr
    pub value: T,
}
```

**原因**：
- 字段名是**有限集合**（ip, time, method 等）
- **高度重复**（1000 条日志共用几十个字段名）
- **长度短**（通常 ≤22 字节，SmolStr 内联存储）
- **性能关键**（频繁 clone）

### 为什么字段值用 ArcStr？
```rust
pub enum Value {
    Chars(ArcStr),       // 字符串值
    Symbol(ArcStr),      // 符号值
    // ...
}
```

**原因**：
- 值的**多样性高**（每条日志的值都不同）
- **可能很长**（如 user_agent 可能 >100 字节）
- **共享收益低**（值通常不重复）
- **ArcStr 适合长字符串**（无长度限制）

### 为什么 Object key 用 ArcStr？
```rust
pub struct ObjectValue(pub BTreeMap<ArcStr, DataField>);
```

**原因**：
- JSON key 可能是**动态的**
- **可能很长**（嵌套路径）
- 使用 BTreeMap 需要 Ord trait（ArcStr 实现了，SmolStr 也实现了）

## wp-lang 迁移建议

### ✅ 可以安全迁移到 FNameStr
wp-lang 的字段名（WplField.meta_name, WplField.name）应该迁移到 FNameStr，因为：
1. **与上游一致** - DataField 使用 FNameStr
2. **类型兼容** - 创建 DataField 时直接传递，无需转换
3. **性能更好** - SmolStr clone 是栈拷贝（≤22字节）

### 当前不兼容的原因
wp-lang 的 parser 返回的是 `DataField`，而 `DataField.name` 已经是 `FNameStr`。
如果 wp-lang 内部使用 `ArcStr` 作为字段名，每次创建 DataField 都需要转换：

```rust
// ❌ 当前（需要转换）
let name: ArcStr = "ip".into();
DataField {
    name: name.into(),  // ArcStr -> SmolStr 转换
    // ...
}

// ✅ 迁移后（直接传递）
let name: FNameStr = "ip".into();
DataField {
    name: name,  // 直接传递
    // ...
}
```

## 结论

**wp-model-core 对字段名的 SmolStr 迁移已完成**，wp-lang 应该跟进迁移以保持一致性和最佳性能。

迁移范围仅限于：
- WplField.meta_name: ArcStr → FNameStr
- WplField.name: Option<ArcStr> → Option<FNameStr>
- 相关 trait 和函数签名

字段值继续使用 ArcStr 是正确的设计。
