// Copyright (c) 2023-, Germano Rizzo <oss /AT/ germanorizzo /DOT/ it>
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::{ops::DerefMut, path::Path as SysPath, time::Duration};

use actix_web::{
    rt::{
        spawn,
        time::{interval_at, sleep, Instant},
    },
    web, Responder,
};
use duckdb::Connection;

use crate::{
    auth::process_creds,
    commons::{abort, delete_old_bkp_dirs, file_exists},
    db_config::Backup,
    main_config::Db,
    req_res::{Response, Token},
    MUTEXES,
};

fn gen_bkp_dir(parent: &str, filepath: &str) -> String {
    let path = SysPath::new(parent);
    let dstpath = SysPath::new(filepath);
    let base_name = dstpath.file_stem().unwrap().to_str().unwrap();
    let intermission = crate::commons::now();

    let new_dir_name = format!("{}_{}", base_name, intermission);
    let new_dir_path = path.join(new_dir_name);

    new_dir_path.into_os_string().into_string().unwrap()
}

fn do_backup(bkp_dir: &str, num_backups: usize, db_path: &str, conn: &Connection) -> Response {
    if !file_exists(bkp_dir) {
        return Response::new_err(404, -1, format!("Backup dir '{}' not found", bkp_dir));
    }

    let dest_dir = gen_bkp_dir(bkp_dir, db_path);
    if file_exists(&dest_dir) {
        Response::new_err(409, -1, format!("Directory '{}' already exists", dest_dir))
    } else {
        let sql = format!("EXPORT DATABASE '{}'", &dest_dir);
        match conn.execute(&sql, []) {
            Ok(_) => match delete_old_bkp_dirs(&dest_dir, num_backups) {
                Ok(_) => Response::new_ok(vec![]),
                Err(e) => Response::new_err(
                    500,
                    -1,
                    format!("Database backed up but error in deleting old files: {}", e),
                ),
            },
            Err(e) => Response::new_err(500, -1, e.to_string()),
        }
    }
}

pub async fn handler(
    db_conf: web::Data<Db>,
    db_name: web::Data<String>,
    token: web::Query<Token>,
) -> impl Responder {
    let db_name = db_name.to_string();
    match &db_conf.conf.backup {
        Some(bkp) => match &bkp.execution.web_service {
            Some(bkp_ws) => {
                if !process_creds(&token.token, &bkp_ws.auth_token, &bkp_ws.hashed_auth_token) {
                    sleep(Duration::from_millis(1000)).await;

                    return Response::new_err(
                        bkp_ws.auth_error_code,
                        -1,
                        format!("In database '{}', backup: token mismatch", db_name),
                    );
                }

                let db_lock = MUTEXES.get().unwrap().get(&db_name).unwrap();
                let mut db_lock_guard = db_lock.lock().unwrap();
                let conn = db_lock_guard.deref_mut();

                do_backup(&bkp.backup_dir, bkp.num_backups, &db_conf.path, conn)
            }
            None => Response::new_err(
                404,
                -1,
                format!(
                    "Database '{}' doesn't have a backup.execution.webService node",
                    db_name
                ),
            ),
        },
        None => Response::new_err(
            404,
            -1,
            format!("Database '{}' doesn't have a backup node", db_name),
        ),
    }
}

pub fn bootstrap_backup(
    is_new_db: bool,
    bkp: &Backup,
    db_name: &String,
    db_path: &str,
    conn: &Connection,
) {
    let bkp = bkp.to_owned();
    let bex = &bkp.execution;
    if bex.on_startup || (is_new_db && bex.on_create) {
        let res = do_backup(&bkp.backup_dir, bkp.num_backups, db_path, conn);

        if !res.success {
            abort(format!(
                "Backup of database '{}': {}",
                db_name,
                res.message.unwrap()
            ));
        }
    }
    println!("  - backup configured");
    if bex.on_create {
        println!("    - performed on database creation");
    }
    if bex.on_startup {
        println!("    - performed on server startup");
    }
    if bex.period > 0 {
        println!("    - performed periodically");
    }
    if bex.web_service.is_some() {
        println!("    - callable via web service");
    }
}

pub fn periodic_backup(bkp: &Backup, db_name: String, db_path: String) {
    let bkp = bkp.to_owned();
    let period = bkp.execution.period;
    let bkp_dir = bkp.backup_dir;
    let num_files = bkp.num_backups;
    if period > 0 {
        let period = period as u64 * 60;
        spawn(async move {
            let p = Duration::from_secs(period);
            let first_start = Instant::now().checked_add(p).unwrap();
            let mut interval = interval_at(first_start, p);

            loop {
                interval.tick().await;

                if !file_exists(&bkp_dir) {
                    eprintln!("Backup dir '{}' not found", bkp_dir);
                    return;
                }

                let dest_dir = gen_bkp_dir(&bkp_dir, &db_path);
                if file_exists(&dest_dir) {
                    eprintln!("Directory '{}' already exists", dest_dir);
                } else {
                    let db_lock = MUTEXES.get().unwrap().get(&db_name).unwrap();
                    let mut db_lock_guard = db_lock.lock().unwrap();
                    let conn = db_lock_guard.deref_mut();

                    let res = do_backup(&bkp_dir, num_files, &db_path, conn);
                    if !res.success {
                        eprintln!("Backing up '{}': {}", db_name, res.message.unwrap())
                    }
                }
            }
        });
    }
}
