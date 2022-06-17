# Factory Integration Tests

There are no actual tests in the typescript file. However, it will console log the key outputs allowing user to see how to interract with the contract on javascript.  

Here are some test written in Typescript using secret.js
To use this test you must run localsecret first. I also recommend following the instructions to increase its speed.
Currently, all the tests are inside `integration.ts`

In order to run the tests first you will need to install the dependencies by running the following command:

```sh
npm install
```

After you have all of the dependencies install you can run the following command in order to build and run your code:

```sh
npx ts-node [[ts_file_name.ts]]
```

You can also choose to debug your code by using the following steps (Using vscode):

1. Press `ctrl+shift+p`
2. Write `JavaScript Debug Terminal` and press `Enter`
3. In the new terminal you can run `npx ts-node [[ts_file_name.ts]]`
4. Your code will be running in debug mode and will stop on every breakpoint placed.
