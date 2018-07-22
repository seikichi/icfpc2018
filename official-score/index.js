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
    while (true) {
        await sleep(3000);
        const stdout = await page.$("#stdout");
        const text = await (await stdout.getProperty("textContent")).jsonValue();

        console.log(text);
        if (text.startsWith("Success::") || text.startsWith("Failure::")) {
            break;
        }
    }
    browser.close();
})();
