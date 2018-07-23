const puppeteer = require('puppeteer');

async function sleep(delay) {
    return new Promise(resolve => setTimeout(resolve, delay));
}

(async() => {
    const task = process.argv[2];
    if (!["assemble", "disassemble", "reassemble"].includes(task)) {
        console.error("invalid argument: please ses the README.md");
        process.exit(1);
    }

    const browser = await puppeteer.launch({
        args: [
            '--no-sandbox',
            '--disable-setuid-sandbox'
        ]
    });
    const page = await browser.newPage();
    await page.goto('https://icfpcontest2018.github.io/full/exec-trace-novis.html');

    // source
    if (task === "assemble") {
        await page.click("#srcModelEmpty");
    } else {
        const target = await page.$("#srcModelFileIn");
        await target.uploadFile("/app/source.mdl");
    }
    // target
    if (task === "disassemble") {
        await page.click("#tgtModelEmpty");
    } else {
        const target = await page.$("#tgtModelFileIn");
        await target.uploadFile("/app/target.mdl");
    }
    // trace
    const trace = await page.$("#traceFileIn");
    await trace.uploadFile("/app/trace.nbt");

    // exec
    await page.click("#execTrace");
    for (var i = 0; ; i++) {
        await sleep(2000);
        const stdout = await page.$("#stdout");
        const text = await (await stdout.getProperty("textContent")).jsonValue();

        if (text.startsWith("Success::") || text.startsWith("Failure::")) {
            var commandsRe = /Commands: *([0-9]+)/;
            var energyRe = /Energy: *([0-9]+)/;
            var m, commands, energy;
            if ((m = commandsRe.exec(text)) !== null) {
                commands = m[1];
            } else {
                commands = -1;
            }
            if ((m = energyRe.exec(text)) !== null) {
                energy = m[1];
            } else {
                energy = -1;
            }
            console.log(JSON.stringify({
                "commands": commands,
                "energy": energy
            }));
            break;
        }

        if (i == 10) {
            console.log("Timeout!");
            break;
        }
    }
    browser.close();
})();
