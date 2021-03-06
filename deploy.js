const config = require('./config.json')
const {promisify} = require('util')
const { exit } = require('process')

const exec = promisify(require('child_process').exec)

const stage = process.env.STAGE || 'local'
const SENDGRID_KEY = process.env.SENDGRID_KEY || 'unset'

const apps = config[stage]

const getActiveServers = async () => JSON.stringify(await Promise.all(apps.map(({name, ty}) => {
    return (async () => {
        const {stdout} = await exec(`heroku apps:info -a ${name} -j`)
        const url = JSON.parse(stdout).app.web_url
        return {
            server_ty: ty,
            url
        }
    })()
})))

const createNewApp = async (server, stage) => {
    if (stage == "local") {
        return exec(`cargo run`)
    }
    const {name} = server
    return exec(`heroku create -a ${name}`)
        .then(() => {
            console.log('adding buildpack...')
            return exec(`heroku buildpacks:add -a ${name} https://github.com/benj2468/heroku-buildpack-rust`)
        })
        .then(() => {
            console.log('adding db...')
            return exec(`heroku addons:create -a ${name} heroku-postgresql --wait`)
        })
        .then(() => {
            return exec(`heroku config:get -a ${name} DATABASE_URL`)
        })
}

const setEnvs = async (activeServers, {name, ty}, stage) => {
    if (stage == 'local') return
    await exec(`heroku config:set -a ${name} HOST=${name}.herokuapp.com`)
    .then(() => exec(`heroku config:set -a ${name} SERVER_TY='"${ty}"'`))
    .then(console.log)
    .then(() => exec(`heroku config:set -a ${name} ACTIVE_SERVERS='${activeServers}'`))
    .then(console.log)
    .then(() => exec(`heroku config:set -a ${name} RUST_CARGO_BUILD_FLAGS="--release --features ${ty.toLowerCase()}"`))
    .then(console.log)
    .then(() => {
        return exec(`heroku config:set -a ${name} SENDGRID_KEY=${SENDGRID_KEY}`)
    })
    .then(console.log)
    .then(() => {
        switch (ty) {
            case 'Email':
                return
            case 'QA':
                return
            case 'Biometric':
                return
            default:
                throw Error("Unimplemented")
        } 
    })
    .then(console.log)
}

const main = async () => {
    await Promise.all(apps.map((server) => {
        return (async () => {
            try {
                await createNewApp(server, stage)
            } catch (e) {
                console.log(`Did not create a new app: ${server.name}, Maybe it already existed: ${e}`)
            }
            
        })()
    }))

    const activeServers = await getActiveServers()

    await Promise.all(apps.map(server => setEnvs(activeServers, server, stage)))

    console.log("Completed ???")
}



main().then(() => exit(0))
