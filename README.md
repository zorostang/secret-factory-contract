# **Secret Factory Contract Template** #

Factory contracts are useful for scaling your web3 app when you need to create new contract instances for various reasons such as launching new games, attaching application specific state to each of your users, etc...

This repo contains two template contracts. The factory contract template is responsible for creating offspring contracts, storing them, and listing them in queries. The factory contract also stores the viewing key of the users so that a user does not need to create multiple viewing keys for each individual offspring they interract with.

This contract makes use of the incubator feature Cashmaps for listing and paging mechanics in factory queries.

## **About the Contracts** ##

The offspring contract is based on the [simple counter template](https://github.com/scrtlabs/secret-template) that everyone should be familiar with. Some additional features were added to it to make it a suitable offspring contract for a factory. Its state now stores some extra fields that are useful for an offspring contract.

The factory registers the offsprings it creates. In order for an offspring to be initialized and registered in the factory, the factory is the one that must be initializing the offspring contract. The registration of the offspring contract is done by a post init callback which carries with it a password to ensure that offspring contracts not initialized by the factory cannot be registered.

The state of the offspring contract has a boolean variable called `active` which is initialized as true. I believe many implementations of the factory model will implement some sense of deactivation/finalization of the offspring contract, such as a finalized auction. That's why offspring are split into two groups in the factory, that is `active` and `inactive`.

Another important feature these contracts implement is that user viewing keys are only stored in factory. So whenever the offspring contract needs to verify that a viewing key is valid, it will query the factory contract (this has no extra gas cost.)

## **Instantiating the Factory Contract** ##

The only data factory template requires is the code id and code hash of the offspring contract. And some entropy is needed for prng seed. The initializer of the factory contract gains admin status.

The following is an example InitMsg:

```json
{
    "entropy": "random_words",
    "offspring_contract": {
        "code_id": 2,
        "code_hash": "D519793AF2623773F46967192C9AFCD9F2E3A2BA0FD927EA6BF3448A723BDE6B"
    }
}
```

## **HandleMsg of the Factory** ##

### **Creating a New Offspring** ###

Creating a new offspring also automatically registers it after the post init callback. The offspring contract requires an initial `count` and an `owner`. This can be called by anyone.

The following is an example message to create an offspring:

```json
{
    "create_offspring": {
        "label": "my_counter",
        "entropy": "random_words",
        "owner": "secret1ap26qrlp8mcq2pg6r47w43l0y8zkqm8a450s03",
        "count": 3,
        "description": "this is the first offspring of this factory."
    }
}
```

|   **Name**  |      **Type**      |                                                **Description**                                                | **Optional** | **Value If Omitted** |
|:-----------:|:------------------:|:-------------------------------------------------------------------------------------------------------------:|:------------:|:--------------------:|
|    label    |       String       | Every contract in secret network can be labelled when initializing. This is the label given to the offspring. |      No      |                      |
|   entropy   |       String       | Used in generating the password which is used to authenticate that offspring was created by this factory      |      No      |                      |
|    owner    | String (HumanAddr) | The user with additional privileges in the offspring.                                                         |      No      |                      |
|    count    |    number (i32)    | The initial count offspring template starts with.                                                             |      No      |                      |
| description |       String       | This string is stored in the offspring. Currently it serves no purpose.                                       |      Yes     |         None         |

### **Updating the Offspring Contract Version** ###

The offspring contract version (code id and code hash) can be updated by the admin. This preserves compatibility with previous versions of the offspring contract, and all new offspring contracts will be in the new version. The following is an example message:

```json
{
    "new_offspring_contract": {
        "offspring_contract": {
            "code_id": 3,
            "code_hash": "6826E1F57AC79DCA02F5DA9AF5879D1314452509D327ECF9975F2CD15D684D91"
        }
    }
}
```

### **Stop/Resume Creation of New Offspring Contracts** ###

The admin may want to freeze the creation of new offspring contracts until its version is updated. The following message is meant to stop the factory creating new offspring.

```json
{
    "set_status": {
        "stop": true
    }
}
```

### **Other Handle Messages** ###

Creating/Setting Viewing Keys work in the expected way. And the other handle messages `deactivate_offspring` and `register_offspring` need to be called by the offspring contract. `deactivate_offspring` is called when the offspring lets the factory know its deactivating and therefore should be moved to the inactive list. `register_offspring` is called as post init callback of the offspring.

## **Queries of the Factory** ##

### **Listing All Active Offspring Information** ###

This returns a list of all active offspring information (which consists of their addresses and labels). There are no optional parameters here.

**Request:**

```json
{"list_active_offspring":{}}
```

| **Name**   | **Type**     | **Description**                               | **Optional** | **Value If Omitted** |
|------------|--------------|-----------------------------------------------|--------------|----------------------|
| start_page | number (u32) | starting page number for the listed offspring |      Yes     |           0          |
|  page_size | number (u32) |   number of offspring to return in this page  |      Yes     |          200         |

**Response:**

```json
{
    "list_active_offspring":{"active":[
        {"address":"secret10pyejy66429refv3g35g2t7am0was7ya6hvrzf","label":"counter1"},
        {"address":"secret1sshdl5qajv0q0k6shlk8m9sd4lplpn6gvf82cx","label":"owner random"}
    ]}
} 
```

### **Listing Inactive Offspring Information** ###

`list_inactive_offspring` query lists inactive offsprings in reverse chronological order. Inactive offspring are indexed by a field called `index`. This is not the same index used to refer to active offspring, this index just reflects their ordering in the inactive list. This index ordering is meant to be customized in your specific use case. This query gives the user a few options on how this list should look like using two optional fields.

**Request:**

| **Name**   | **Type**     | **Description**                               | **Optional** | **Value If Omitted** |
|------------|--------------|-----------------------------------------------|--------------|----------------------|
| start_page | number (u32) | starting page number for the listed offspring |      Yes     |           0          |
|  page_size | number (u32) |   number of offspring to return in this page  |      Yes     |          200         |

**Response:**

```json
{
    "list_inactive_offspring": {
        "inactive": [
            {
                "address": "secret10pyejy66429refv3g35g2t7am0was7ya6hvrzf",
                "label": "counter1"
            }
        ]
    }
}
```

### **List My Offspring** ###

`list_my_offspring` lists all active and inactive offspring that an address owns. For this, you need to provide an address and its valid viewing key. The filter option allows the user to list only active, only inactive, or both.

**Request:**

| **Name**    | **Type**                              | **Description**                                                                                 | **Optional** | **Value If Omitted** |
|-------------|---------------------------------------|-------------------------------------------------------------------------------------------------|--------------|----------------------|
|   address   |           String (HumanAddr)          |                             the address whose offspring are queried                             |      No      |                      |
| viewing_key |                 String                |                                    viewing key of the address                                   |      No      |                      |
|    filter   | one of "active", "inactive", or "all" |                      filter for listing only active or inactive offspring.                      |      Yes     |         "all"        |
|  start_page |              number (u32)             | starting page number for the listed offspring (individually for both active and inactive lists) |      Yes     |           0          |
|  page_size  |              number (u32)             |                            number of offspring to return in this page                           |      Yes     |          200         |

**Response:**

```json
{
    "list_my_offspring":{
        "active":[{"address":"secret1vjecguu37pmd577339wrdp208ddzymku0apnlw","label":"my_counter2"}],
        "inactive":[{"label":"counter1","address":"secret10pyejy66429refv3g35g2t7am0was7ya6hvrzf"}]
    }
}
```

### **IsKeyValid** ##

`is_key_valid` query can be used by anyone that wants to check whether a given address and viewing key pair match in the factory contract. The offspring contracts query this method when they need to verify a user's viewing keys. There are no optional parameters here.

**Request:**

```json
{
    "is_key_valid": {
        "address": "secret1ap26qrlp8mcq2pg6r47w43l0y8zkqm8a450s03",
        "viewing_key": "viewing_key"
    }
}
```

**Response:**

```json
{"is_key_valid":{"is_valid":true}}
```

## **HandleMsg of the Offspring** ##

It has the same basic handle messages that [simple counter template](https://github.com/scrtlabs/secret-template) has. So I will not list them. There is only one additional handle message unique to the offspring template.

### **Deactivate** ###

This message is meant to deactivate the offspring contract and can only be called by the owner of the offspring. Once deactivated, a contract cannot be reactivated. This handle message also has to let the factory know to move the offspring from active to inactive storage.

```json
{"deactivate":{}}
```

## **Queries of the Offspring** ##

There is only one query of the offspring contact which is inherited from [simple counter template](https://github.com/scrtlabs/secret-template). The only difference in its implementation is that only someone with the viewing key of the owner of the offspring can query the count. This query queries the factory to validate the viewing key. There are no optional fields.

**Request:**

```json
{
    "get_count":{
        "address": "address_of_owner",
        "viewing_key": "viewing key of address"
    }
}
```

**Response:**

```json
{"count_response":{"count":2}}
```
