use anyhow::Result;
use rusqlite::Connection;
use sea_query::{Cond, Expr, Index, Query, SqliteQueryBuilder, Table};
use sea_query_rusqlite::RusqliteBinder;
use std::path::PathBuf;

use crate::data::{
    CacheMeta, LabelFilter, LabelFiltersTable, PrFilter, PullRequest, PullRequestsTable,
    CACHE_VERSION,
};

pub fn get_cache_path() -> Option<PathBuf> {
    dirs::config_dir().map(|p| p.join("ghui").join("cache.db"))
}

pub fn init_db(conn: &Connection) -> Result<()> {
    use sea_query::ColumnDef;

    // Create version table
    let cache_meta_sql = Table::create()
        .table(CacheMeta::Table)
        .if_not_exists()
        .col(
            ColumnDef::new(CacheMeta::Key)
                .text()
                .not_null()
                .primary_key(),
        )
        .col(ColumnDef::new(CacheMeta::Value).integer())
        .build(SqliteQueryBuilder);
    conn.execute(&cache_meta_sql, [])?;

    // Check version
    let (version_sql, version_values) = Query::select()
        .column(CacheMeta::Value)
        .from(CacheMeta::Table)
        .and_where(Expr::col(CacheMeta::Key).eq("version"))
        .build_rusqlite(SqliteQueryBuilder);

    let current_version: Option<i32> = conn
        .query_row(&version_sql, &*version_values.as_params(), |row| row.get(0))
        .ok();

    if current_version != Some(CACHE_VERSION) {
        // Clear old cache
        let drop_pr_sql = Table::drop()
            .table(PullRequestsTable::Table)
            .if_exists()
            .build(SqliteQueryBuilder);
        let _ = conn.execute(&drop_pr_sql, []);

        let drop_labels_sql = Table::drop()
            .table(LabelFiltersTable::Table)
            .if_exists()
            .build(SqliteQueryBuilder);
        let _ = conn.execute(&drop_labels_sql, []);

        // Upsert version
        let (upsert_sql, upsert_values) = Query::insert()
            .into_table(CacheMeta::Table)
            .columns([CacheMeta::Key, CacheMeta::Value])
            .values_panic(["version".into(), CACHE_VERSION.into()])
            .on_conflict(
                sea_query::OnConflict::column(CacheMeta::Key)
                    .update_column(CacheMeta::Value)
                    .to_owned(),
            )
            .build_rusqlite(SqliteQueryBuilder);
        conn.execute(&upsert_sql, &*upsert_values.as_params())?;
    }

    // Create pull_requests table
    let sql = Table::create()
        .table(PullRequestsTable::Table)
        .if_not_exists()
        .col(
            sea_query::ColumnDef::new(PullRequestsTable::Number)
                .integer()
                .not_null(),
        )
        .col(
            sea_query::ColumnDef::new(PullRequestsTable::Title)
                .text()
                .not_null(),
        )
        .col(
            sea_query::ColumnDef::new(PullRequestsTable::Branch)
                .text()
                .not_null(),
        )
        .col(
            sea_query::ColumnDef::new(PullRequestsTable::RepoOwner)
                .text()
                .not_null(),
        )
        .col(
            sea_query::ColumnDef::new(PullRequestsTable::RepoName)
                .text()
                .not_null(),
        )
        .col(
            sea_query::ColumnDef::new(PullRequestsTable::CiStatus)
                .text()
                .not_null(),
        )
        .col(
            sea_query::ColumnDef::new(PullRequestsTable::Filter)
                .text()
                .not_null(),
        )
        .col(
            sea_query::ColumnDef::new(PullRequestsTable::Author)
                .text()
                .not_null()
                .default(""),
        )
        .primary_key(
            Index::create()
                .col(PullRequestsTable::Number)
                .col(PullRequestsTable::RepoOwner)
                .col(PullRequestsTable::RepoName)
                .col(PullRequestsTable::Filter),
        )
        .build(SqliteQueryBuilder);
    conn.execute(&sql, [])?;

    // Create label_filters table
    let label_sql = Table::create()
        .table(LabelFiltersTable::Table)
        .if_not_exists()
        .col(
            sea_query::ColumnDef::new(LabelFiltersTable::Id)
                .integer()
                .not_null()
                .auto_increment()
                .primary_key(),
        )
        .col(
            sea_query::ColumnDef::new(LabelFiltersTable::LabelName)
                .text()
                .not_null(),
        )
        .col(sea_query::ColumnDef::new(LabelFiltersTable::RepoOwner).text())
        .col(sea_query::ColumnDef::new(LabelFiltersTable::RepoName).text())
        .build(SqliteQueryBuilder);
    conn.execute(&label_sql, [])?;

    // Create unique index on label_filters
    let index_sql = Index::create()
        .if_not_exists()
        .name("idx_label_filters_unique")
        .table(LabelFiltersTable::Table)
        .col(LabelFiltersTable::LabelName)
        .col(LabelFiltersTable::RepoOwner)
        .col(LabelFiltersTable::RepoName)
        .unique()
        .build(SqliteQueryBuilder);
    conn.execute(&index_sql, [])?;

    Ok(())
}

pub fn load_cache(owner: &str, repo: &str, filter: PrFilter) -> Result<Vec<PullRequest>> {
    let path = get_cache_path().ok_or_else(|| anyhow::anyhow!("No cache dir"))?;
    if !path.exists() {
        return Ok(Vec::new());
    }

    let conn = Connection::open(&path)?;
    init_db(&conn)?;

    let (sql, values) = Query::select()
        .columns([
            PullRequestsTable::Number,
            PullRequestsTable::Title,
            PullRequestsTable::Branch,
            PullRequestsTable::RepoOwner,
            PullRequestsTable::RepoName,
            PullRequestsTable::CiStatus,
            PullRequestsTable::Author,
        ])
        .from(PullRequestsTable::Table)
        .and_where(Expr::col(PullRequestsTable::RepoOwner).eq(owner))
        .and_where(Expr::col(PullRequestsTable::RepoName).eq(repo))
        .and_where(Expr::col(PullRequestsTable::Filter).eq(filter.to_str()))
        .build_rusqlite(SqliteQueryBuilder);

    let mut stmt = conn.prepare(&sql)?;
    let prs = stmt
        .query_map(&*values.as_params(), |row| {
            Ok(PullRequest {
                number: row.get::<_, i64>(0)? as u64,
                title: row.get(1)?,
                branch: row.get(2)?,
                repo_owner: row.get(3)?,
                repo_name: row.get(4)?,
                ci_status: row.get::<_, String>(5)?.parse().unwrap(),
                author: row.get(6)?,
                head_sha: None, // Not cached, will be populated on fresh fetch
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    Ok(prs)
}

pub fn save_cache(prs: &[PullRequest], owner: &str, repo: &str, filter: PrFilter) -> Result<()> {
    let path = get_cache_path().ok_or_else(|| anyhow::anyhow!("No cache dir"))?;

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let conn = Connection::open(&path)?;
    init_db(&conn)?;

    // Only delete PRs for this specific repo and filter
    let (delete_sql, delete_values) = Query::delete()
        .from_table(PullRequestsTable::Table)
        .and_where(Expr::col(PullRequestsTable::RepoOwner).eq(owner))
        .and_where(Expr::col(PullRequestsTable::RepoName).eq(repo))
        .and_where(Expr::col(PullRequestsTable::Filter).eq(filter.to_str()))
        .build_rusqlite(SqliteQueryBuilder);

    conn.execute(&delete_sql, &*delete_values.as_params())?;

    for pr in prs {
        let (insert_sql, insert_values) = Query::insert()
            .into_table(PullRequestsTable::Table)
            .columns([
                PullRequestsTable::Number,
                PullRequestsTable::Title,
                PullRequestsTable::Branch,
                PullRequestsTable::RepoOwner,
                PullRequestsTable::RepoName,
                PullRequestsTable::CiStatus,
                PullRequestsTable::Filter,
                PullRequestsTable::Author,
            ])
            .values_panic([
                (pr.number as i64).into(),
                (&pr.title).into(),
                (&pr.branch).into(),
                (&pr.repo_owner).into(),
                (&pr.repo_name).into(),
                pr.ci_status.to_str().into(),
                filter.to_str().into(),
                (&pr.author).into(),
            ])
            .build_rusqlite(SqliteQueryBuilder);

        conn.execute(&insert_sql, &*insert_values.as_params())?;
    }

    Ok(())
}

pub fn load_label_filters(owner: &str, repo: &str) -> Result<Vec<LabelFilter>> {
    let path = get_cache_path().ok_or_else(|| anyhow::anyhow!("No cache dir"))?;
    if !path.exists() {
        return Ok(Vec::new());
    }

    let conn = Connection::open(&path)?;
    init_db(&conn)?;

    // Load both repo-specific labels and global labels
    let (sql, values) = Query::select()
        .columns([
            LabelFiltersTable::Id,
            LabelFiltersTable::LabelName,
            LabelFiltersTable::RepoOwner,
            LabelFiltersTable::RepoName,
        ])
        .from(LabelFiltersTable::Table)
        .cond_where(
            Cond::any()
                .add(
                    Cond::all()
                        .add(Expr::col(LabelFiltersTable::RepoOwner).eq(owner))
                        .add(Expr::col(LabelFiltersTable::RepoName).eq(repo)),
                )
                .add(
                    Cond::all()
                        .add(Expr::col(LabelFiltersTable::RepoOwner).is_null())
                        .add(Expr::col(LabelFiltersTable::RepoName).is_null()),
                ),
        )
        .order_by_expr(
            Expr::col(LabelFiltersTable::RepoOwner).is_null(),
            sea_query::Order::Asc,
        )
        .order_by(LabelFiltersTable::LabelName, sea_query::Order::Asc)
        .build_rusqlite(SqliteQueryBuilder);

    let mut stmt = conn.prepare(&sql)?;
    let labels = stmt
        .query_map(&*values.as_params(), |row| {
            Ok(LabelFilter {
                id: row.get(0)?,
                label_name: row.get(1)?,
                repo_owner: row.get(2)?,
                repo_name: row.get(3)?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    Ok(labels)
}

pub fn save_label_filter(label_name: &str, owner: Option<&str>, repo: Option<&str>) -> Result<()> {
    use sea_query::OnConflict;

    let path = get_cache_path().ok_or_else(|| anyhow::anyhow!("No cache dir"))?;

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let conn = Connection::open(&path)?;
    init_db(&conn)?;

    let owner_value: sea_query::SimpleExpr = match owner {
        Some(s) => s.into(),
        None => sea_query::Keyword::Null.into(),
    };
    let repo_value: sea_query::SimpleExpr = match repo {
        Some(s) => s.into(),
        None => sea_query::Keyword::Null.into(),
    };

    let (sql, values) = Query::insert()
        .into_table(LabelFiltersTable::Table)
        .columns([
            LabelFiltersTable::LabelName,
            LabelFiltersTable::RepoOwner,
            LabelFiltersTable::RepoName,
        ])
        .values_panic([label_name.into(), owner_value, repo_value])
        .on_conflict(OnConflict::new().do_nothing().to_owned())
        .build_rusqlite(SqliteQueryBuilder);

    conn.execute(&sql, &*values.as_params())?;

    Ok(())
}

pub fn delete_label_filter(id: i64) -> Result<()> {
    let path = get_cache_path().ok_or_else(|| anyhow::anyhow!("No cache dir"))?;
    if !path.exists() {
        return Ok(());
    }

    let conn = Connection::open(&path)?;
    init_db(&conn)?;

    let (sql, values) = Query::delete()
        .from_table(LabelFiltersTable::Table)
        .and_where(Expr::col(LabelFiltersTable::Id).eq(id))
        .build_rusqlite(SqliteQueryBuilder);

    conn.execute(&sql, &*values.as_params())?;

    Ok(())
}
