# WPL 管道函数（更新建议）

下列内容概述了 `take()` + 字段别名的使用方式，便于复制到正式文档。

## 函数分类（替换原有表格）

| 类型 | 前缀 | 说明 |
|------|------|------|
| **字段集合操作** | `f_` / `_` / 无前缀别名 | `f_` 函数按字段名查找；`_` 或无前缀别名依赖活动字段（需先使用 `take()`、`last()` 等 selector）|
| **直接函数** | 无前缀 | 直接对当前数据进行处理 |

## 函数概览（替换原有概览表）

| 函数名/别名 | 参数 | 说明 |
|---------------|------|------|
| `f_has` / `has` | 0 或 1 | `f_has(field)` 针对指定字段；`has()` 在当前活动字段上判断是否存在 |
| `f_chars_has` / `chars_has` | 1 或 2 | 字符串相等判断 |
| `f_chars_not_has` / `chars_not_has` | 1 或 2 | 字符串不等判断 |
| `f_chars_in` / `chars_in` | 1 或 2 | 字符串是否属于指定列表 |
| `f_digit_has` / `digit_has` | 1 或 2 | 数值相等判断 |
| `f_digit_in` / `digit_in` | 1 或 2 | 数值是否属于指定列表 |
| `f_ip_in` / `ip_in` | 1 或 2 | IP 是否属于列表 |

> 旧 DSL 继续支持 `f_` 写法，字段名写 `_` 时等价于“使用当前活动字段”。无前缀别名是 `_` 写法的语义化包装，推荐与 `take()` 链式使用。

## `f_has` / `has`

**语法：**
```wpl
f_has(<field_name>)
has()
```
**参数：**
- `field_name`（可选）：指定字段使用 `f_has`；若省略则需先 `take()` 选定活动字段，再调用 `has()`。

**示例：**
```wpl
rule check_field {
  (json | f_has(age))
}

rule check_field_in_pipe {
  (json(chars@code) | take(code) | has())
}
```

## `f_chars_has` / `chars_has`

**语法：**
```wpl
f_chars_has(<field_name>, <pattern>)
chars_has(<pattern>)
```
**参数：**
- `field_name`：旧语法使用；若省略则需先 `take()` 选定目标字段。
- `<pattern>`：要匹配的字符串。

**示例：**
```wpl
rule check_error {
  (json | f_chars_has(message, error))
}

rule check_error_field_version {
  (json(chars@msg) | take(msg) | chars_has(error))
}
```

其它 `f_chars_*`、`f_digit_*`、`f_ip_in` 的“字段别名”用法可类比：
1. 先 `take(<field>)` 或 `last()` 激活目标字段；
2. 使用无前缀别名（如 `digit_in([1,2,3])`）。

这样可以避免在管道中频繁写字段名，并与 `take()/last()` 选择器组合，语义更清晰。
