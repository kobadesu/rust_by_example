

    #[macro_use]
    extern crate mysql;
    extern crate r2d2_mysql;
    extern crate r2d2;

    use std::env;
    use std::sync::Arc;
    use mysql::{Opts, OptsBuilder};
    use r2d2_mysql::MysqlConnectionManager;

    const DATABASE_HOST: &str = "DATABASE_HOST";
    const DATABASE_PORT: &str = "DATABASE_PORT";
    const DATABASE_USER: &str = "DATABASE_USER";
    const DATABASE_PASS: &str = "DATABASE_PASS";
    const DATABASE_NAME: &str = "DATABASE_NAME";

    const DATABASE_POOL_SIZE: u32 = 4;

    fn main() {
    println!("Hello, world!");
        // 環境変数の読み取りとチェック
        // パラメタの読み取りはコマンドライン引数からでも良いですが、
        // 名前から値を引こうとすると別途ライブラリが必要になるため、環境変数を使っています
        let db_host = env_var(DATABASE_HOST, Some("127.0.0.1".to_string()));
        let db_port = env_var(DATABASE_PORT, Some("3306".to_string()));
        let db_user = env_var(DATABASE_USER, Some("".to_string()));
        let db_pass = env_var(DATABASE_PASS, Some("".to_string()));
        let db_name = env_var(DATABASE_NAME, Some("".to_string()));

        assert_ne!(db_host, "");
        assert_ne!(db_port, "");
        assert_ne!(db_user, "");
        assert_ne!(db_pass, "");
        assert_ne!(db_name, "");

        // ビルダークラスには各パラメタをひとつずつ設定していくインターフェースも用意されていますが、
        // ビルダー型が&mutになるためMysqlConnectionManager::newへそのまま渡せなくなり使い勝手が悪いので、
        // 接続URLを組み立ててOpts::from_urlに渡す形にしています
        let db_url = format!(
            "mysql://{user}:{pass}@{host}:{port}/{name}",
            user = db_user,
            pass = db_pass,
            host = db_host,
            port = db_port,
            name = db_name
        );

        let opts = Opts::from_url(&db_url).unwrap();
        let builder = OptsBuilder::from_opts(opts);
        let manager = MysqlConnectionManager::new(builder);

        // マルチスレッドでプールからコネクションを取り出すような使い方を想定してArcでラップします
        let pool = Arc::new(r2d2::Pool::builder()
            .max_size(DATABASE_POOL_SIZE)
            .build(manager).unwrap());

        let items = query_table_outline(pool, db_name);
println!("{:?}", items);
        for item in items {
            println!(
                "TableOutline\n\
                table_name: {}\n\
                table_comment: {}\n\
                table_fqn: {}\n",
                item.table_name,
                item.table_comment.unwrap(),
                item.table_fqn
            );
        }
    }

    fn env_var(name: &str, def_var: Option<String>) -> String {
        let env_var = env::var(name);
        return match def_var {
            Some(v) => env_var.unwrap_or(v),
            _ => env_var.expect(format!("{} must be set", name).as_str()),
        }
    }

    fn query_table_outline(
        pool: Arc<r2d2::Pool<MysqlConnectionManager>>,
        param: String
    ) -> Vec<TableOutline> {
        // 並列アクセス可能なようにプールへの参照をカウントアップ
        let pool = pool.clone();

        // プールからコネクションを取り出す
        let mut conn = pool.get().unwrap();

        // PreparedQueryを発行
        // :param_schema_name のようにコロンを頭につけると名前付きのプレースホルダーになります
        return conn.prep_exec(r#"
                SELECT
                    tbl.table_name      AS table_name,
                    tbl.table_comment   AS table_comment,
                    CONCAT(tbl.table_schema, '.', tbl.table_name)
                                        AS table_fqn
                FROM
                    information_schema.tables tbl
                WHERE
                    tbl.table_schema <> :param_schema_name
                ORDER BY
                    tbl.table_name
                LIMIT 5
                "#, params!{
                    "param_schema_name" => param
                })
            .map::<Vec<TableOutline>, _>(|result| {
                result
                    .map(|x| x.unwrap())
                    .map(|row| {
                        // タプルに結果セットの行をマッピング
                        let (table_name, table_comment, table_fqn) = mysql::from_row(row);

                        // 戻り値型に詰め替え
                        TableOutline {
                            table_name,
                            table_comment,
                            table_fqn,
                        }
                    }).collect()
            }).unwrap();
    }

    #[derive(Debug, PartialEq, Eq)]
    struct TableOutline {
        table_name: String,
        table_comment: Option<String>,
        table_fqn: String,
    }
