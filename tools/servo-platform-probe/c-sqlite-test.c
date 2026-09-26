#include <sqlite3.h>
extern int infinity_sqlite_initialize(void);
// ------------------------=
// FUNC: infinity_c_sqlite_test
// DESC: Exercises real SQL transactions, rollback, session isolation and denied persistent paths.
// ------------------=
int infinity_c_sqlite_test(void) {
    if(infinity_sqlite_initialize())return 1;
    sqlite3 *db=0;sqlite3_stmt *row=0;
    if(sqlite3_open(":memory:",&db))return 2;
    if(sqlite3_exec(db,"CREATE TABLE items(value INTEGER); INSERT INTO items VALUES(42); BEGIN; INSERT INTO items VALUES(7); ROLLBACK;",0,0,0))return 3;
    if(sqlite3_prepare_v2(db,"SELECT count(*),sum(value),length(randomblob(16)) FROM items",-1,&row,0))return 4;
    if(sqlite3_step(row)!=SQLITE_ROW || sqlite3_column_int(row,0)!=1 || sqlite3_column_int(row,1)!=42 || sqlite3_column_int(row,2)!=16)return 5;
    if(sqlite3_finalize(row) || sqlite3_close(db))return 6;
    db=0;if(sqlite3_open(":memory:",&db))return 7;
    if(sqlite3_prepare_v2(db,"SELECT * FROM items",-1,&row,0)!=SQLITE_ERROR)return 8;
    if(sqlite3_close(db))return 9;
    db=0;int status=sqlite3_open("/should-not-exist.sqlite",&db);
    if(status!=SQLITE_CANTOPEN)return 10;
    if(db && sqlite3_close(db))return 11;
    return sqlite3_shutdown();
}
