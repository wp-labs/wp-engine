use criterion::{black_box, criterion_group, criterion_main, Criterion};
use arcstr::ArcStr;
use smol_str::SmolStr;

/// 模拟问题：每次都创建新的 ArcStr，没有共享
fn arcstr_no_sharing(c: &mut Criterion) {
    c.bench_function("ArcStr - 每次新建（错误用法）", |b| {
        b.iter(|| {
            let mut fields = Vec::new();
            for _ in 0..100 {
                // ❌ 每次都创建新的 Arc，没有共享！
                let name: ArcStr = "ip".into();
                fields.push(name.clone());  // clone 时是不同的 Arc
            }
            black_box(fields);
        });
    });
}

/// 正确用法：共享同一个 ArcStr 实例
fn arcstr_with_sharing(c: &mut Criterion) {
    c.bench_function("ArcStr - 共享实例（正确用法）", |b| {
        // ✅ 只创建一次，后续共享
        let shared_name: ArcStr = "ip".into();

        b.iter(|| {
            let mut fields = Vec::new();
            for _ in 0..100 {
                fields.push(shared_name.clone());  // 共享同一个 Arc，很快！
            }
            black_box(fields);
        });
    });
}

/// String 基准测试
fn string_baseline(c: &mut Criterion) {
    c.bench_function("String - 每次新建", |b| {
        b.iter(|| {
            let mut fields = Vec::new();
            for _ in 0..100 {
                let name = String::from("ip");
                fields.push(name.clone());
            }
            black_box(fields);
        });
    });
}

/// SmolStr 测试
fn smolstr_test(c: &mut Criterion) {
    c.bench_function("SmolStr - 每次新建", |b| {
        b.iter(|| {
            let mut fields = Vec::new();
            for _ in 0..100 {
                let name: SmolStr = "ip".into();
                fields.push(name.clone());
            }
            black_box(fields);
        });
    });
}

/// 模拟实际场景：解析 1000 条日志，每条 20 个字段
fn realistic_scenario(c: &mut Criterion) {
    // 预先创建共享的 ArcStr（正确用法）
    let field_names_shared: Vec<ArcStr> = vec![
        "ip".into(), "time".into(), "method".into(), "path".into(),
        "status".into(), "size".into(), "referrer".into(), "user_agent".into(),
    ];

    c.bench_function("实际场景 - ArcStr 共享", |b| {
        b.iter(|| {
            let mut all_fields = Vec::new();
            for _log in 0..1000 {
                for name in &field_names_shared {
                    all_fields.push(name.clone());  // 原子递增
                }
            }
            black_box(all_fields);
        });
    });

    c.bench_function("实际场景 - ArcStr 不共享", |b| {
        b.iter(|| {
            let mut all_fields = Vec::new();
            for _log in 0..1000 {
                for _ in 0..8 {
                    let name: ArcStr = "ip".into();  // 每次新建！
                    all_fields.push(name);
                }
            }
            black_box(all_fields);
        });
    });

    c.bench_function("实际场景 - String", |b| {
        b.iter(|| {
            let mut all_fields = Vec::new();
            for _log in 0..1000 {
                for _ in 0..8 {
                    all_fields.push(String::from("ip"));
                }
            }
            black_box(all_fields);
        });
    });

    // SmolStr 实际场景测试
    let field_names_smol: Vec<SmolStr> = vec![
        "ip".into(), "time".into(), "method".into(), "path".into(),
        "status".into(), "size".into(), "referrer".into(), "user_agent".into(),
    ];

    c.bench_function("实际场景 - SmolStr 预创建", |b| {
        b.iter(|| {
            let mut all_fields = Vec::new();
            for _log in 0..1000 {
                for name in &field_names_smol {
                    all_fields.push(name.clone());  // 栈拷贝（≤22字节）
                }
            }
            black_box(all_fields);
        });
    });

    c.bench_function("实际场景 - SmolStr 每次创建", |b| {
        b.iter(|| {
            let mut all_fields = Vec::new();
            for _log in 0..1000 {
                for _ in 0..8 {
                    let name: SmolStr = "ip".into();
                    all_fields.push(name);
                }
            }
            black_box(all_fields);
        });
    });
}

criterion_group!(
    benches,
    arcstr_no_sharing,
    arcstr_with_sharing,
    string_baseline,
    smolstr_test,
    realistic_scenario
);
criterion_main!(benches);
