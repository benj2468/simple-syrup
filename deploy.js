const {apps} = require('./config.json')
const {promisify} = require('util')
const { stderr, exit } = require('process')

const exec = promisify(require('child_process').exec)

const stage = process.env.STAGE || 'staging'


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
        
}

const main = async () => {
    await Promise.all(apps.map(({name, ty}) => {
        return (async () => {
            var proceed = true
            try {
                await createNewApp(name, stage)
            } catch (e) {
                if (!e.stderr.includes(`Name ${name} is already taken`)) proceed = false;
            }

            if (proceed)
            {
                await exec(`heroku config:get -a ${name} DATABASE_URL`)
                    .then(({stdout: url}) => {
                        const serverConfig = JSON.stringify([{
                            server_ty: ty,
                            db_options: {
                                url: url.trim()
                            }
                        }])
                        return exec(`heroku config:set -a ${name} SERVERS_CONFIG='${serverConfig}'`)
                    })
            }
            
        })()
    }))

    for(const {name} of apps) {
        await exec(`git push https://git.heroku.com/${name}.git HEAD:main`)
    }

    console.log("Completed ✅")
}



main().then(() => exit(0))
