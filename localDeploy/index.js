const {exec} = require('child_process');
const {local} = require('../config');
const Web3 = require('web3')

const getHost = (i) => `http://127.0.0.1:${getPort(i)}`
const getPort = (i) => 8080 + i
const dbName = (name) => `cpass${name}`

const web3 = new Web3(process.env.WEB3_URL || "http://127.0.0.1:7545");

const makeDbs = async () => {
    return Promise.all(local.map(({name}) => {
        return new Promise((resolve, reject) => {
            exec(`createdb ${dbName(name)}`, (err, stdout, stderr) => {
                resolve()
            })
        })
    }))
}

const main = async () => {
    const accounts = await web3.eth.getAccounts();

    const ACTIVE_SERVERS = local.map(({name, ty}, i) => {
        return {
            server_ty: ty,
            url: getHost(i)
        }
    })

    await makeDbs()

    local.forEach(({name, ty}, i) => {

        const SERVER_TY = `"${ty}"`;
        const WEB3_HOST = "ws://127.0.0.1:7545"
    
        const ETH_ADDRESS = accounts[i]

        const HOST = "http://127.0.0.1"
        const PORT = getPort(i)

        const DATABASE_URL = `postgres://127.0.0.1:5432/${dbName(name)}`

        const cmd = `cargo run --features ${ty.toLowerCase()} --features web3`
        
        const child = exec(cmd, {
            env: {
                ...process.env,
                WEB3_HOST,
                ETH_ADDRESS,
                SERVER_TY,
                DATABASE_URL,
                HOST,
                PORT,
                BIOMETRIC_API_URL: 'http://127.0.0.1:8000',
                ACTIVE_SERVERS: JSON.stringify(ACTIVE_SERVERS)
            }
        }, (err, stdout, stderr) => {
            if (err) {
                console.log(stderr)
            }
            console.log(stdout)
        });

        child.stdout.on('data', (data) => {
            console.log(`[${i}] ${data}`);
        });
    })
}

main()