const {apps} = require('./config.json')
const {promisify} = require('util')
const { exit } = require('process')

const exec = promisify(require('child_process').exec)

const stage = process.env.STAGE || 'staging'
const SENDGRID_KEY = process.env.SENDGRID_KEY || 'unset'

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
        .then(({stdout: url}) => {
            const serverConfig = JSON.stringify([{
                server_ty: ty,
                db_options: {
                    uri: url.trim()
                }
            }])
            return exec(`heroku config:set -a ${name} SERVERS_CONFIG='${serverConfig}'`)
        })
}

const setEnvs = async ({name, ty}) => {
    exec(`heroku config:set -a ${name} HOST=${name}.herokuapp.com`)
    .then(() => {
        switch (ty) {
            case 'Email':
                return exec(`heroku config:set -a ${name} SENDGRID_KEY=${SENDGRID_KEY}`)
            default:
                throw Error("Unimplemented")
        } 
    })
}

const main = async () => {
    await Promise.all(apps.map((server) => {
        return (async () => {
            try {
                await createNewApp(server.name, stage)
            } catch (e) {
                
            }
            await setEnvs(server)
        })()
    }))

    for(const {name} of apps) {
        console.log(`Pushing ${name} ...`)
        await exec(`git push https://git.heroku.com/${name}.git HEAD:main`)
        console.log(`Completed ${name} ✅`)
    }

    console.log("Completed ✅")
}



main().then(() => exit(0))
