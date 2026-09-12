#!/usr/bin/env node

const path = require("node:path");
const { evaluateFiles } = require("../src");

function printUsage() {
  console.error("Usage:");
  console.error("  apex-lite evaluate <intent.json> <policy.yaml> [--log <file>]");
}

function parseArguments(argv) {
  if (argv[0] !== "evaluate") {
    throw new Error("Expected the 'evaluate' command.");
  }

  if (argv.length !== 3 && argv.length !== 5) {
    throw new Error("Incorrect number of arguments.");
  }

  const command = {
    intentPath: path.resolve(argv[1]),
    policyPath: path.resolve(argv[2]),
    logPath: null
  };

  if (argv.length === 5) {
    if (argv[3] !== "--log") {
      throw new Error("Only the --log flag is supported.");
    }

    command.logPath = path.resolve(argv[4]);
  }

  return command;
}

function run(argv) {
  const command = parseArguments(argv);
  const { receipt } = evaluateFiles(command.intentPath, command.policyPath, {
    logPath: command.logPath
  });

  console.log(JSON.stringify(receipt, null, 2));
  return receipt;
}

module.exports = {
  parseArguments,
  run
};

if (require.main === module) {
  try {
    run(process.argv.slice(2));
  } catch (error) {
    console.error(error.message);
    printUsage();
    process.exit(1);
  }
}
