package main

import (
	"encoding/json"
	"fmt"
	"io/ioutil"
)

// Opcode6502 defines the 6502 opcodes
type Opcode6502 struct {
	Name        string         `json:"name"`
	Description string         `json:"description"`
	Flags       Flags6502      `json:"flags"`
	AddressMode []AddrMode6502 `json:"addressingModes"`
	Type        string         `json:"type"`
	Expression  string         `json:"expression"`
	Assignee    string         `json:"assignee"`
}

// Flags6502 defines flags in 6502
type Flags6502 struct {
	Overflow  int `json:"overflow,omitempty"`
	Carry     int `json:"carry,omitempty"`
	Zero      int `json:"zero,omitempty"`
	Negative  int `json:"negative,omitempty"`
	Decimal   int `json:"decimal,omitempty"`
	Interrupt int `json:"interrupt,omitempty"`
}

// AddrMode6502 defines addressing mode in 6502
type AddrMode6502 struct {
	Opcode        string `json:"opcode"`
	Cycles        int    `json:"cycles"`
	Mode          int    `json:"mode"`
	CycleModifier int    `"cycleModifier"`
}

func check(e error) {
	if e != nil {
		panic(e)
	}
}

func main() {
	data, err := ioutil.ReadFile("6502.json")
	check(err)

	var response []Opcode6502
	json.Unmarshal(data, &response)

	for _, opcode := range response {
		fmt.Printf("%s - %s\n", opcode.Name, opcode.Description)
	}

}
