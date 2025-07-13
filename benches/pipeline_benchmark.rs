use criterion::{black_box, criterion_group, criterion_main, Criterion};
use etlrs::prelude::*;
use etlrs::transform::common::AggregateFunction;
use std::collections::HashMap;
use std::io::Write;
use tempfile::NamedTempFile;

fn benchmark_csv_processing(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    
    c.bench_function("csv_extract_1000_rows", |b| {
        b.iter(|| {
            runtime.block_on(async {
                // Cria arquivo CSV temporário com 1000 linhas
                let mut temp_file = NamedTempFile::new().unwrap();
                writeln!(temp_file, "id,name,age,active").unwrap();
                
                for i in 0..1000 {
                    writeln!(temp_file, "{},User{},{},true", i, i, 20 + (i % 50)).unwrap();
                }
                
                let extractor = CsvExtractor::new(temp_file.path());
                let result = extractor.extract().await.unwrap();
                
                black_box(result);
            });
        })
    });
}

fn benchmark_json_processing(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    
    c.bench_function("json_extract_1000_rows", |b| {
        b.iter(|| {
            runtime.block_on(async {
                // Cria arquivo JSON temporário com 1000 registros
                let mut temp_file = NamedTempFile::new().unwrap();
                let mut data = Vec::new();
                
                for i in 0..1000 {
                    data.push(serde_json::json!({
                        "id": i,
                        "name": format!("User{}", i),
                        "age": 20 + (i % 50),
                        "active": true
                    }));
                }
                
                writeln!(temp_file, "{}", serde_json::to_string(&data).unwrap()).unwrap();
                
                let extractor = JsonExtractor::new(temp_file.path());
                let result = extractor.extract().await.unwrap();
                
                black_box(result);
            });
        })
    });
}

fn benchmark_transformations(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    
    c.bench_function("filter_transform_1000_rows", |b| {
        b.iter(|| {
            runtime.block_on(async {
                // Cria dados de teste
                let mut data = Vec::new();
                for i in 0..1000 {
                    let mut row = HashMap::new();
                    row.insert("id".to_string(), DataValue::Integer(i));
                    row.insert("age".to_string(), DataValue::Integer(20 + (i % 50)));
                    row.insert("active".to_string(), DataValue::Boolean(i % 2 == 0));
                    data.push(row);
                }
                
                let transform = FilterTransform::new(|row| {
                    row.get("age")
                        .and_then(|v| v.as_integer())
                        .unwrap_or(0) >= 30
                });
                
                let result = transform.transform(data).await.unwrap();
                
                black_box(result);
            });
        })
    });
    
    c.bench_function("map_transform_1000_rows", |b| {
        b.iter(|| {
            runtime.block_on(async {
                // Cria dados de teste
                let mut data = Vec::new();
                for i in 0..1000 {
                    let mut row = HashMap::new();
                    row.insert("id".to_string(), DataValue::Integer(i));
                    row.insert("name".to_string(), DataValue::String(format!("User{}", i)));
                    data.push(row);
                }
                
                let transform = MapTransform::new(|mut row| {
                    row.insert("processed".to_string(), DataValue::Boolean(true));
                    row.insert("timestamp".to_string(), DataValue::String(chrono::Utc::now().to_rfc3339()));
                    row
                });
                
                let result = transform.transform(data).await.unwrap();
                
                black_box(result);
            });
        })
    });
}

fn benchmark_pipeline_end_to_end(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    
    c.bench_function("pipeline_csv_to_json_1000_rows", |b| {
        b.iter(|| {
            runtime.block_on(async {
                // Cria arquivo CSV temporário
                let mut csv_file = NamedTempFile::new().unwrap();
                writeln!(csv_file, "id,name,age,active").unwrap();
                
                for i in 0..1000 {
                    writeln!(csv_file, "{},User{},{},true", i, i, 20 + (i % 50)).unwrap();
                }
                
                // Cria arquivo JSON de saída
                let json_file = NamedTempFile::new().unwrap();
                
                let pipeline = Pipeline::builder()
                    .extract(CsvExtractor::new(csv_file.path()))
                    .transform(FilterTransform::new(|row| {
                        row.get("age")
                            .and_then(|v| v.as_integer())
                            .unwrap_or(0) >= 25
                    }))
                    .load(JsonLoader::new(json_file.path()))
                    .build();
                
                let result = pipeline.execute().await.unwrap();
                
                black_box(result);
            });
        })
    });
}

fn benchmark_parallel_processing(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    
    c.bench_function("parallel_transform_1000_rows", |b| {
        b.iter(|| {
            runtime.block_on(async {
                // Cria dados de teste
                let mut data = Vec::new();
                for i in 0..1000 {
                    let mut row = HashMap::new();
                    row.insert("id".to_string(), DataValue::Integer(i));
                    row.insert("value".to_string(), DataValue::Integer(i * 2));
                    data.push(row);
                }
                
                let base_transform = MapTransform::new(|mut row| {
                    if let Some(DataValue::Integer(val)) = row.get("value") {
                        row.insert("squared".to_string(), DataValue::Integer(val * val));
                    }
                    row
                });
                
                let parallel_transform = ParallelTransform::new(base_transform, 4);
                let result = parallel_transform.transform(data).await.unwrap();
                
                black_box(result);
            });
        })
    });
}

fn benchmark_memory_usage(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    
    c.bench_function("memory_loader_1000_rows", |b| {
        b.iter(|| {
            runtime.block_on(async {
                // Cria dados de teste
                let mut data = Vec::new();
                for i in 0..1000 {
                    let mut row = HashMap::new();
                    row.insert("id".to_string(), DataValue::Integer(i));
                    row.insert("data".to_string(), DataValue::String(format!("Data{}", i)));
                    data.push(row);
                }
                
                let loader = MemoryLoader::new();
                let result = loader.load(data).await.unwrap();
                
                black_box(result);
            });
        })
    });
}

fn benchmark_aggregations(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    
    c.bench_function("aggregate_transform_1000_rows", |b| {
        b.iter(|| {
            runtime.block_on(async {
                // Cria dados de teste
                let mut data = Vec::new();
                for i in 0..1000 {
                    let mut row = HashMap::new();
                    row.insert("category".to_string(), DataValue::String(format!("Cat{}", i % 10)));
                    row.insert("value".to_string(), DataValue::Integer(i));
                    data.push(row);
                }
                
                let mut aggregations = HashMap::new();
                aggregations.insert("value".to_string(), AggregateFunction::Sum);
                aggregations.insert("value".to_string(), AggregateFunction::Count);
                
                let transform = AggregateTransform::new(
                    vec!["category".to_string()],
                    aggregations,
                );
                
                let result = transform.transform(data).await.unwrap();
                
                black_box(result);
            });
        })
    });
}

criterion_group!(
    benches,
    benchmark_csv_processing,
    benchmark_json_processing,
    benchmark_transformations,
    benchmark_pipeline_end_to_end,
    benchmark_parallel_processing,
    benchmark_memory_usage,
    benchmark_aggregations
);

criterion_main!(benches);
