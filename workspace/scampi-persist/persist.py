from pymongo import MongoClient
from constants import *
import json, sys, os

# Connect to the database
client = MongoClient(CONNECTION_STRING)
db = client[DB_NAME]

crate_collection = db[CRATE_COLL_NAME]
fn_collection = db[FN_COLL_NAME]
invoc_collection = db[INVOC_COLL_NAME]

def read(path: str) -> dict:
    file = open(path, 'r')
    return json.load(file)

def main():
    args = sys.argv

    # Name of the crate being processed
    krate = args[1]

    # Path to the directory containing function records
    fn_dir = f"data/{krate}/functions"

    # Path to the directory containing invocation records
    invoc_dir = f"data/{krate}/invocations"

    # Upsert all invocations
    for invoc_file in os.listdir(invoc_dir):
        invocs = read(os.path.join(invoc_dir, invoc_file))

        crate_name = invoc_file.split(".")[0]
        crate = { "name": crate_name }

        crate_collection.update_one(
            filter=crate,
            update={ '$set': crate },
            upsert=True
        )

        for invoc in invocs:
            invoc['crate'] = crate_name

            invoc_collection.update_one(
                filter=invoc,
                update={ '$set': invoc },
                upsert=True
            )

    # Upsert all functions
    for fn_file in os.listdir(fn_dir):
        fns = read(os.path.join(fn_dir, fn_file))

        for (fn_name, fn_data) in fns.items():
            fn_collection.update_one(
                filter={ 'name': fn_name },
                update={ '$set': fn_data },
                upsert=True
            )

if __name__ == "__main__":
    main()