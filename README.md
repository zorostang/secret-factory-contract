# secret-factory-contract

this is a factory contract for creating child contracts. useful for scaling your web3 app when you need to create new instances for various reasons such as launching new games, attaching application specific state to each of your users, etc...

InitMsg:

```json
{
    "entropy": "random words",
    "offspring_contract": {
        "code_id": 2,
        "code_hash": "7241C420431B275229E62213062678A59623D0BC52E5427E15A10262DFE2D0B6"
    }
}
```
