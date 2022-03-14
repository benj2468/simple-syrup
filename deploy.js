const {apps} = require('./config.json')
const {promisify} = require('util')
const { exit } = require('process')

const exec = promisify(require('child_process').exec)

const stage = process.env.STAGE || 'staging'
const SENDGRID_KEY = process.env.SENDGRID_KEY || 'unset'

const activeServers = JSON.stringify(apps.map(({name, ty}) => ({
    server_ty: ty,
    url: `https://${name}.herokuapp.com`
})))

const createNewApp = async (name, stage) => {
    return exec(`heroku create -a ${name}`)
        .then(() => {
            console.log('adding buildpack...')
            return exec(`heroku buildpacks:add -a ${name} heroku-community/multi-procfile`)
        })
        .then(() => {
            console.log('adding buildpack...')
            return exec(`heroku buildpacks:add -a ${name} emk/rust`)
        })
        .then(() => {
            console.log('setting procfile...')
            return exec(`heroku config:set -a ${name} PROCFILE=Procfile`)
        })
        .then(() => {
            console.log('setting pipeline...')
            return exec(`heroku pipelines:add cpass -a ${name} -s ${stage}`)
        })
        .then(() => {
            console.log('adding db...')
            return exec(`heroku addons:create -a ${name} heroku-postgresql --wait`)
        })
        .then(() => {
            return exec(`heroku config:get -a ${name} DATABASE_URL`)
        })
}

const setEnvs = async ({name, ty}) => {
    exec(`heroku config:set -a ${name} HOST=${name}.herokuapp.com`)
    .then(() => exec(`heroku config:set -a ${name} SERVER_TY="${ty}"`))
    .then(() => exec(`heroku config:set -a ${name} ACTIVE_SERVERS='${activeServers}'`))
    .then(() => {
        switch (ty) {
            case 'Email':
                return exec(`heroku config:set -a ${name} SENDGRID_KEY=${SENDGRID_KEY}`)
            default:
                throw Error("Unimplemented")
        } 
    })
    .catch(console.log)
}

const main = async () => {
    await Promise.all(apps.map((server) => {
        return (async () => {
            try {
                await createNewApp(server.name, stage)
            } catch (e) {
                console.log(`Did not create a new app: ${server.name}, Maybe it already existed`)
            } finally {
                await setEnvs(server)
            }
            
        })()
    }))

    console.log("Completed ✅")
}



main().then(() => exit(0))
