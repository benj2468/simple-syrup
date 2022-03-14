const fs = require('fs')

const serversCount = process.env.SERVER_COUNT

const PROC_FILE = './Procfile'

const main = () => {
    fs.writeFileSync(PROC_FILE, '')
    for (let i = 0; i < serversCount; i++) {
        fs.appendFileSync(PROC_FILE, `cpass${i}: SERVER_ID=${i} ./target/release/simple-syrup\n`) 
    }
}

main()

